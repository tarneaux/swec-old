use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Watcher {
    /// Information about the service, for humans
    pub info: Info,
    /// Status history of the service
    pub statuses: VecDeque<Status>,
}

impl Watcher {
    #[must_use]
    /// Create a new watcher with an empty history.
    pub fn new(info: Info, hist_len: usize) -> Self {
        Self {
            info,
            statuses: VecDeque::with_capacity(hist_len),
        }
    }
}

/// Information about a service. Only intended to be read by humans.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Info {
    /// Description of the service
    pub description: String,
    /// URL of the service, if applicable
    pub url: Option<String>,
    // TODO: service groups with a Group struct
}

impl Info {
    #[must_use]
    pub const fn new(description: String, url: Option<String>) -> Self {
        Self { description, url }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Status {
    /// Whether the service is up or down
    pub is_up: bool,
    /// Human readable information about the status
    pub message: String,
    /// The time the status was recorded
    pub time: DateTime<Local>,
}
