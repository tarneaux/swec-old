use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Watcher<Buffer: StatusBuffer> {
    /// Information about the service, for humans
    pub spec: Spec,
    /// Status history of the service
    pub statuses: Buffer,
}

impl<Buffer: StatusBuffer> Watcher<Buffer> {
    #[must_use]
    /// Create a new watcher with an empty history.
    pub fn new(spec: Spec, buf: Buffer) -> Self {
        Self {
            spec,
            statuses: buf,
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

pub trait StatusBuffer {
    fn push(&mut self, status: (DateTime<Local>, Status));
    fn get(&self, index: usize) -> Option<(DateTime<Local>, Status)>;
    fn len(&self) -> usize;
}

pub type VecBuffer = Vec<(DateTime<Local>, Status)>;

impl StatusBuffer for VecBuffer {
    fn push(&mut self, status: (DateTime<Local>, Status)) {
        self.push(status);
    }

    fn get(&self, index: usize) -> Option<(DateTime<Local>, Status)> {
        self.as_slice().get(index).cloned()
    }

    fn len(&self) -> usize {
        self.len()
    }
}

pub type BTreeMapBuffer = BTreeMap<DateTime<Local>, Status>;

impl StatusBuffer for BTreeMapBuffer {
    fn push(&mut self, status: (DateTime<Local>, Status)) {
        self.insert(status.0, status.1);
    }

    fn get(&self, index: usize) -> Option<(DateTime<Local>, Status)> {
        self.iter()
            .nth(index)
            .and_then(|(time, status)| Some((time.clone(), status.clone())))
    }

    fn len(&self) -> usize {
        self.len()
    }
}
