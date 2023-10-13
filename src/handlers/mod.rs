/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

pub mod histfile;
use crate::watchers::{TimeStampedStatus, Watcher};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(
        &self,
        statuses: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
        watchers: &'_ [Watcher],
    );
    async fn shutdown(&self) {}
    fn get_name(&self) -> &str;
}
