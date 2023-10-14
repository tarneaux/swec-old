/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use super::{TimeStampedStatus, Watcher};
use crate::handlers::Handler;
use futures::future::join_all;
use signal_hook::consts::{SIGINT, SIGQUIT, SIGTERM};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::{JoinError, JoinSet};

pub struct WatcherPond {
    pub watchers: Vec<Watcher>,
    pub status_histories: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
    pub histsize: usize,
    pub interval: Duration,
    pub handlers: Vec<Box<dyn Handler>>,
    pub is_stopping: Arc<AtomicBool>,
}

impl WatcherPond {
    pub fn new(
        watchers: Vec<Watcher>,
        histsize: usize,
        interval: Duration,
        handlers: Vec<Box<dyn Handler>>,
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

        let is_stopping = Arc::new(AtomicBool::new(false));
        for signal in &[SIGINT, SIGTERM, SIGQUIT] {
            let is_stopping = is_stopping.clone();
            signal_hook::flag::register(*signal, is_stopping).unwrap_or_else(|e| {
                panic!("Failed to register signal handler: {e:?}");
            });
        }

        Self {
            watchers,
            status_histories,
            histsize,
            interval,
            handlers,
            is_stopping,
        }
    }

    pub async fn watch(&mut self) {
        loop {
            let min_time = self.interval;

            let min_time_handle = tokio::spawn(async move {
                tokio::time::sleep(min_time).await;
            });

            if self.shutdown_if_needed().await {
                break;
            }

            match self.run_all_watchers(self.interval).await {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Error while running watcher: {e:?}");
                }
            }

            self.run_all_handlers().await;

            if self.shutdown_if_needed().await {
                break;
            }

            // Wait for the interval to pass so that we don't
            // change the frequency of checks
            min_time_handle.await.unwrap_or_else(|e| {
                eprintln!("Error while waiting for end of interval: {e:?}");
            });
        }
    }

    async fn shutdown_if_needed(&mut self) -> bool {
        if self.is_stopping.load(Ordering::Relaxed) {
            self.shutdown().await;
            true
        } else {
            false
        }
    }

    pub async fn shutdown(&mut self) {
        eprintln!("Pond: shutting down all handlers...");
        for handler in &self.handlers {
            handler.shutdown().await;
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
                let history = &mut self.status_histories.write().await[id];
                if history.len() == self.histsize {
                    history.remove(0);
                }
                history.push(status);
            }
        }
        Ok(())
    }

    async fn run_all_handlers(&self) {
        join_all(
            self.handlers
                .iter()
                .map(|handler| handler.handle(self.status_histories.clone(), &self.watchers)),
        )
        .await;
    }
}
