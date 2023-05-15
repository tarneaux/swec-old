use crate::watcher::{ServiceWatcher, Status};
use std::fmt::Debug;
use std::time::Duration;
use tokio::task::{JoinError, JoinSet};

pub struct ServiceWatcherPond {
    pub watchers: Vec<ServiceWatcher>,
}

impl ServiceWatcherPond {
    pub fn new() -> Self {
        Self {
            watchers: Vec::new(),
        }
    }

    pub async fn run(&self, timeout: Duration) -> Result<Vec<Status>, PondWorkerError> {
        let mut join_set = JoinSet::new();

        for (id, watcher) in self.watchers.iter().enumerate() {
            let watcher = watcher.clone();
            join_set.spawn(async move { (watcher.get_current_status(&timeout).await, id) });
        }

        let mut statuses = Vec::with_capacity(self.watchers.len()); // TODO: resize in the same
                                                                    // line
        statuses.resize(self.watchers.len(), None);
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
        Ok(statuses.into_iter().map(|s| s.unwrap()).collect()) // Unwrap is safe (see above)
    }
}

#[derive(Debug)]
pub enum PondWorkerError {
    JoinError(JoinError), // TODO This is quite unclean. Maybe we should also store the id of the
    // watcher that failed?
    DidNotFinish,
}

trait Name {}
