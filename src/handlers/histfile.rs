/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use super::Handler;
use crate::watchers::{TimeStampedStatus, Watcher, WatcherPond};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncSeekExt;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;

pub struct HistfileHandler {
    pub buf_writer: Arc<RwLock<BufWriter<File>>>,
}

impl HistfileHandler {
    pub fn new(buf_writer: BufWriter<File>) -> Self {
        let buf_writer = Arc::new(RwLock::new(buf_writer));
        Self { buf_writer }
    }

    async fn handle_async(
        &self,
        statuses: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
        watchers: &[Watcher],
    ) -> Result<(), HistfileError> {
        let statuses = statuses.read().await;

        // Get a hashmap of the watchers and their status history
        let statuses_map: Vec<HistoryWithWatcher> = watchers
            .iter()
            .enumerate()
            .map(|(i, watcher)| HistoryWithWatcher {
                watcher: watcher.clone(),
                history: statuses[i].clone(),
            })
            .collect();

        let statuses = serde_json::to_string(&statuses_map).map_err(HistfileError::SerdeError)?;

        {
            let mut buf_writer = self.buf_writer.write().await;
            buf_writer
                .seek(std::io::SeekFrom::Start(0))
                .await
                .map_err(HistfileError::IoError)?;
            buf_writer
                .write_all(statuses.as_bytes())
                .await
                .map_err(HistfileError::IoError)?;
        }
        Ok(())
    }
}

#[async_trait]
impl Handler for HistfileHandler {
    async fn handle(
        &self,
        statuses: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
        watchers: &'_ [Watcher],
    ) {
        self.handle_async(statuses, watchers)
            .await
            .unwrap_or_else(|e| {
                eprintln!("Error while writing histfile: {e}");
            });
    }

    async fn shutdown(&self) {
        self.buf_writer
            .write()
            .await
            .shutdown()
            .await
            .unwrap_or_else(|e| {
                eprintln!("Error while shutting down histfile handler: {e}");
            });
    }

    fn get_name(&self) -> &str {
        "histfile"
    }
}

pub fn read_histories_from_file(path: &str) -> Result<Vec<HistoryWithWatcher>, HistfileError> {
    let file = std::fs::File::open(path).map_err(HistfileError::IoError)?;
    let histories: Vec<HistoryWithWatcher> =
        serde_json::from_reader(file).map_err(HistfileError::SerdeError)?;
    Ok(histories)
}

pub async fn restore_histories_to_pond(
    histories: Vec<HistoryWithWatcher>,
    pond: WatcherPond,
) -> WatcherPond {
    let pond = pond;
    {
        let mut status_histories = pond.status_histories.write().await;
        for history in histories {
            if let Some(id) = pond
                .watchers
                .iter()
                .position(|watcher| watcher == &history.watcher)
            {
                status_histories[id] = history.history;
            } else {
                eprintln!(
                    "Warning: watcher {:?} from histfile was not found in pond, ignoring",
                    history.watcher
                );
            }
        }
    }
    pond
}

#[derive(Debug)]
pub enum HistfileError {
    IoError(tokio::io::Error),
    SerdeError(serde_json::Error),
}

impl std::fmt::Display for HistfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "HistfileError: {e}"),
            Self::SerdeError(e) => write!(f, "HistfileError: {e}"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct HistoryWithWatcher {
    watcher: Watcher,
    history: Vec<TimeStampedStatus>,
}
