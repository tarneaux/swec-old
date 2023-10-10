/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use crate::status_handlers::StatusHandler;
use crate::watcher::Status;
use async_trait::async_trait;
use serde_json;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::RwLock;

pub struct HistfileStatusHandler {
    pub buf_writer: Arc<RwLock<BufWriter<File>>>,
}

impl HistfileStatusHandler {
    pub fn new(buf_writer: BufWriter<File>) -> Self {
        Self {
            buf_writer: Arc::new(RwLock::new(buf_writer)),
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

        let mut buf_writer = self.buf_writer.write().await;
        buf_writer
            .write_all(statuses.as_bytes())
            .await
            .map_err(|e| HistfileError::IoError(e))?;
        buf_writer.flush().await.unwrap();
        Ok(())
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
    IoError(tokio::io::Error),
}

impl std::fmt::Display for HistfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistfileError::IoError(e) => write!(f, "HistfileError: {}", e),
        }
    }
}
