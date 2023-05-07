use crate::watcher::{LineParseError, LineParseErrorKind, ServiceWatcher, Status};
use std::fmt::Debug;
use std::io::BufRead;
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

    pub fn new_from_stdin() -> Result<Self, LineParseError> {
        let mut pond = Self::new();
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = line.map_err(|e| LineParseError {
                line: e.to_string(),
                kind: LineParseErrorKind::IoError,
            })?;
            pond.add_watcher_from_line(&line)?;
        }
        Ok(pond)
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

    pub fn add_watcher_from_line(&mut self, line: &str) -> Result<(), LineParseError> {
        let mut fields = line.split_whitespace();
        let name = fields.next().ok_or(LineParseError {
            line: line.to_string(),
            kind: LineParseErrorKind::TooFewFields,
        })?;

        let line_without_name = fields.collect::<Vec<&str>>().join(" ");
        let watcher = ServiceWatcher::from_line(&line_without_name)?;

        self.add_watcher(name.to_string(), watcher)
            .map_err(|_| LineParseError {
                line: line.to_string(),
                kind: LineParseErrorKind::TooManyFields,
            })
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
    pub name: String,
    pub status: Status,
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
