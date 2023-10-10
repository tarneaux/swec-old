use crate::status_handlers::StatusHandler;
use crate::watcher::Status;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;

pub struct HistfileStatusHandler {
    pub writer: Arc<RwLock<BufWriter<File>>>,
}

impl HistfileStatusHandler {
    pub fn new(writer: BufWriter<File>) -> Self {
        Self {
            writer: Arc::new(RwLock::new(writer)),
        }
    }

    async fn handle_async(
        &self,
        statuses: Arc<RwLock<Vec<Vec<Status>>>>,
    ) -> Result<(), HistfileError> {
        let statuses = statuses.read().await;
        let statuses = statuses
            .iter()
            .map(|v| v.last().unwrap().clone())
            .collect::<Vec<Status>>();
        let statuses = serde_json::to_string(&statuses).unwrap();

        let mut writer = self
            .writer
            .try_write()
            .map_err(|_| HistfileError::AlreadyLocked)?;
        writer
            .write_all(statuses.as_bytes())
            .await
            .map_err(|e| HistfileError::IoError(e))?;
        writer.flush().await.unwrap();
        return Ok(());
    }
}

#[async_trait]
impl StatusHandler for HistfileStatusHandler {
    async fn handle(&self, statuses: Arc<RwLock<Vec<Vec<Status>>>>) {
        self.handle_async(statuses).await.unwrap_or_else(|e| {
            eprintln!("Error while writing histfile: {}", e);
        })
    }

    fn get_name(&self) -> &str {
        "histfile"
    }
}

#[derive(Debug)]
pub enum HistfileError {
    AlreadyLocked,
    IoError(tokio::io::Error),
}

impl std::fmt::Display for HistfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistfileError::AlreadyLocked => {
                write!(f, "HistfileError: Already locked. This could indicate too many concurrent writes to the histfile.")
            }
            HistfileError::IoError(e) => write!(f, "HistfileError: {}", e),
        }
    }
}
