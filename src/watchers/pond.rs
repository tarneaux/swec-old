/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use super::{TimeStampedStatus, Watcher};
use crate::handlers::Handler;
use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::{JoinError, JoinSet};

pub struct WatcherPond {
    pub watchers: Vec<Watcher>,
    pub status_histories: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
    pub histsize: usize,
    pub interval: Duration,
    pub status_handlers: Vec<Box<dyn Handler>>,
}

impl WatcherPond {
    pub fn new(
        watchers: Vec<Watcher>,
        histsize: usize,
        interval: Duration,
        status_handlers: Vec<Box<dyn Handler>>,
    ) -> Self {
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
            status_handlers,
        }
    }

    pub async fn watch(&mut self) -> tokio::task::JoinHandle<()> {
        loop {
            let min_time = self.interval;

            let min_time_handle = tokio::spawn(async move {
                tokio::time::sleep(min_time).await;
            });

            match self.run_all_watchers(self.interval).await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error while running watcher: {:?}", e);
                }
            }

            self.run_all_status_handlers().await;

            // Wait for the interval to pass so that we don't
            // change the frequency of checks
            min_time_handle.await.unwrap_or_else(|e| {
                eprintln!("Error while waiting for timeout: {:?}", e);
            });
        }
    }

    async fn run_all_watchers(&mut self, timeout: Duration) -> Result<(), JoinError> {
        let mut join_set = JoinSet::new();

        for (id, watcher) in self.watchers.iter().enumerate() {
            let watcher = watcher.clone();
            join_set.spawn(async move {
                let current_status = watcher.get_current_status(&timeout).await;
                (TimeStampedStatus::new_now(current_status), id)
            });
        }

        loop {
            let (status, id) = match join_set.join_next().await {
                Some(v) => v,
                None => break,
            }?;
            {
                let status_histories = &mut self.status_histories.write().await;
                let history = &mut status_histories[id];
                if history.len() == self.histsize {
                    history.remove(0);
                }
                history.push(status);
            }
        }
        Ok(())
    }

    async fn run_all_status_handlers(&self) {
        join_all(
            self.status_handlers
                .iter()
                .map(|handler| handler.handle(self.status_histories.clone(), &self.watchers)),
        )
        .await;
    }
}
