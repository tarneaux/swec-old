use crate::watcher::{ServiceWatcher, Status};
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};

pub struct ServiceWatcherPond {
    named_watchers: Vec<NamedWatcher>,
}

impl ServiceWatcherPond {
    pub fn new() -> Self {
        Self {
            named_watchers: Vec::new(),
        }
    }

    pub fn add_watcher(
        &mut self,
        name: String,
        watcher: ServiceWatcher,
    ) -> Result<(), NameAlreadyTakenError> {
        if self.named_watchers.iter().any(|w| w.name == name) {
            Err(NameAlreadyTakenError { name })
        } else {
            self.named_watchers.push(NamedWatcher {
                watcher,
                name,
                id: self.named_watchers.len(),
            });
            Ok(())
        }
    }

    pub async fn run(&self, timeout: Duration) -> Result<Vec<NamedWatcherStatus>, PondWorkerError> {
        let mut join_set = JoinSet::new();

        for named_watcher in self.named_watchers.iter() {
            let named_watcher = named_watcher.clone();
            join_set.spawn(async move {
                let status = named_watcher.watcher.get_current_status(&timeout).await;
                (
                    NamedWatcherStatus {
                        name: named_watcher.name,
                        status,
                    },
                    named_watcher.id,
                )
            });
        }

        let mut statuses = Vec::with_capacity(self.named_watchers.len());
        statuses.resize(self.named_watchers.len(), None);
        loop {
            match join_set.join_next().await {
                Some(status) => match status {
                    Ok(status) => statuses[status.1] = Some(status.0),
                    Err(e) => return Err(PondWorkerError::JoinError(e)),
                },
                None => break,
            }
        }
        if statuses.iter().any(|s| s.is_none()) {
            return Err(PondWorkerError::DidNotFinish);
        }
        Ok(statuses.into_iter().map(|s| s.unwrap()).collect())
    }
}

#[derive(Clone)]
struct NamedWatcher {
    watcher: ServiceWatcher,
    name: String,
    id: usize,
}

#[derive(Debug, Clone)]
pub struct NamedWatcherStatus {
    name: String,
    status: Status,
}

#[derive(Debug)]
pub struct NameAlreadyTakenError {
    name: String,
}

#[derive(Debug)]
pub enum PondWorkerError {
    JoinError(JoinError),
    DidNotFinish,
}
