use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

mod ringbuffer;

pub use ringbuffer::RingBuffer;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Watcher {
    /// Information about the service, for humans
    pub spec: Spec,
    /// Status history of the service
    pub statuses: RingBuffer<(DateTime<Local>, Status)>,
}

impl Watcher {
    #[must_use]
    /// Create a new watcher with an empty history.
    pub fn new(spec: Spec, hist_len: usize) -> Self {
        Self {
            spec,
            statuses: RingBuffer::new(hist_len),
        }
    }
}

/// Information about a service. Only intended to be read by humans.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Spec {
    /// Description of the service
    pub description: String,
    /// URL of the service, if applicable
    pub url: Option<String>,
    // TODO: service groups with a Group struct
}

impl Spec {
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
}
