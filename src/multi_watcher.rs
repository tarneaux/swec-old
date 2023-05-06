use crate::watcher::{ServiceWatcher, Status};
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};

pub struct ServiceWatcherPond {
    watchers: Vec<ServiceWatcher>,
}

impl ServiceWatcherPond {
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    pub fn add_watcher(&mut self, watcher: ServiceWatcher) {
        self.watchers.push(watcher);
    }

    pub async fn run(&self, timeout: Duration) -> Result<Vec<Status>, JoinError> {
        let mut join_set = JoinSet::new();
        for watcher in self.watchers.iter() {
            let watcher = watcher.clone();
            join_set.spawn(async move { watcher.get_current_status(&timeout).await });
        }

        // while let Some(Status) = join_set.join_next().await {}
        // Get all the statuses and return them
        let mut statuses = Vec::new();
        loop {
            match join_set.join_next().await {
                Some(status) => match status {
                    Ok(status) => statuses.push(status),
                    Err(e) => return Err(e),
                },
                None => break,
            }
        }
        Ok(statuses)
    }
}

#[derive(Debug)]
pub struct LockingError;
