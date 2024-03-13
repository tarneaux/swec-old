use crate::{checker, Spec};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub writable: bool,
    pub swec_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    UpdatedSpec(checker::Spec),
    AddedStatus(DateTime<Local>, checker::Status),
    Initial(Spec, Option<(DateTime<Local>, checker::Status)>),
    CheckerDeleted,
}
