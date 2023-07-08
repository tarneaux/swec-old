use crate::watcher::{ServiceWatcher, Status};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};

pub struct ServiceWatcherPond {
    pub watchers: Vec<ServiceWatcher>,
    pub statushistories: Arc<RwLock<Vec<Vec<Status>>>>,
    pub histsize: usize,
    pub interval: Duration,
}

impl ServiceWatcherPond {
    fn new(watchers: Vec<ServiceWatcher>, histsize: usize, interval: Duration) -> Self {
        let mut statushistories = Vec::with_capacity(watchers.len());
        // We immediately allocate the maximum amount of memory that we will need for the history
        // of each watcher. This way:
        //   - There is no need to reallocate memory in each iteration
        //   - The user will have no surprises: the memory usage (at least for the history) will be
        //   constant
        // +1: because we will be rolling the history (meaning we will add a new element and remove
        // the oldest one in each iteration)
        statushistories.resize(watchers.len(), Vec::with_capacity(histsize + 1));

        let statushistories = Arc::new(RwLock::new(statushistories));
        Self {
            watchers,
            statushistories,
            histsize,
            interval,
        }
    }

    pub fn new_from_config(path: &str) -> Result<Self, ConfigReadingError> {
        let config = Config::read(path)?;
        Ok(Self::new(config.watchers, config.histsize, config.interval))
    }

    async fn run_once(&mut self, timeout: Duration) -> Result<(), JoinError> {
        let mut join_set = JoinSet::new();

        for (id, watcher) in self.watchers.iter().enumerate() {
            let watcher = watcher.clone();
            join_set.spawn(async move { (watcher.get_current_status(&timeout).await, id) });
        }

        loop {
            let (status, id) = match join_set.join_next().await {
                Some(v) => v,
                None => break,
            }?;
            {
                let statushistories = &mut self.statushistories.write();
                let history = &mut statushistories[id];
                if history.len() == self.histsize {
                    history.remove(0);
                }
                history.push(status);
            }
        }
        Ok(())
    }

    pub fn start_watcher(&mut self) -> tokio::task::JoinHandle<()> {
        let mut copied_self = self.clone();
        tokio::spawn(async move {
            loop {
                let timeout_handle = tokio::spawn(async move {
                    tokio::time::sleep(copied_self.interval).await;
                });

                match copied_self.run_once(copied_self.interval).await {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error while running watcher: {:?}", e);
                    }
                }

                // Wait for the interval to pass so that we don't
                // change the frequency of checks
                timeout_handle.await.unwrap();
            }
        })
    }
}

impl Clone for ServiceWatcherPond {
    fn clone(&self) -> Self {
        Self {
            watchers: self.watchers.clone(),
            statushistories: self.statushistories.clone(),
            histsize: self.histsize,
            interval: self.interval,
        }
    }
}

trait Name {}

#[derive(Serialize, Deserialize)]
struct Config {
    pub watchers: Vec<ServiceWatcher>,
    pub interval: Duration,
    pub histsize: usize,
}

impl Config {
    pub fn read(path: &str) -> Result<Self, ConfigReadingError> {
        let file = std::fs::File::open(path).map_err(ConfigReadingError::FileError)?;
        let config: Self = serde_yaml::from_reader(file).map_err(ConfigReadingError::YamlError)?;
        Ok(config)
    }
}

#[derive(Debug)]
pub enum ConfigReadingError {
    FileError(std::io::Error),
    YamlError(serde_yaml::Error),
}
