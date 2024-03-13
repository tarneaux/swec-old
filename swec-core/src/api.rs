use crate::{watcher, Spec};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub writable: bool,
    pub swec_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    UpdatedSpec(watcher::Spec),
    AddedStatus(DateTime<Local>, watcher::Status),
    Initial(Spec, Option<(DateTime<Local>, watcher::Status)>),
    WatcherDeleted,
}
