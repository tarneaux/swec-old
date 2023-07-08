use crate::watcher::{ServiceWatcher, Status};
use core::fmt::{Display, Formatter};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};

pub struct ServiceWatcherPond {
    pub watchers: Vec<ServiceWatcher>,
    pub status_histories: Arc<RwLock<Vec<Vec<Status>>>>,
    pub histsize: usize,
    pub interval: Duration,
}

impl ServiceWatcherPond {
    fn new(watchers: Vec<ServiceWatcher>, histsize: usize, interval: Duration) -> Self {
        let mut status_histories = Vec::with_capacity(watchers.len());
        // We immediately allocate the maximum amount of memory that we will need for the history
        // of each watcher. This way:
        //   - There is no need to reallocate memory in each iteration
        //   - The user will have no surprises: the memory usage will be constant for the history
        // +1: because we will be rolling the history (meaning we will add a new element and remove
        // the oldest one in each iteration)
        status_histories.resize(watchers.len(), Vec::with_capacity(histsize + 1));

        let status_histories = Arc::new(RwLock::new(status_histories));
        Self {
            watchers,
            status_histories,
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
                let status_histories = &mut self.status_histories.write();
                let history = &mut status_histories[id];
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
                timeout_handle.await.unwrap_or_else(|e| {
                    eprintln!("Error while waiting for timeout: {:?}", e);
                });
            }
        })
    }
}

impl Clone for ServiceWatcherPond {
    fn clone(&self) -> Self {
        Self {
            watchers: self.watchers.clone(),
            status_histories: self.status_histories.clone(),
            histsize: self.histsize,
            interval: self.interval,
        }
    }
}

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

impl Display for ConfigReadingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError(e) => write!(f, "Error while reading config file: {}", e),
            Self::YamlError(e) => write!(f, "Error while parsing config file: {}", e),
        }
    }
}
