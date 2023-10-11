/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

pub mod histfile;
use crate::watcher::{ServiceWatcher, Status};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait StatusHandler {
    async fn handle(&self, statuses: Arc<RwLock<Vec<Vec<Status>>>>, watchers: &Vec<ServiceWatcher>);
    fn get_name(&self) -> &str;
}
