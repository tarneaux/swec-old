use chrono::{DateTime, Local};
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Watcher<Buffer: StatusBuffer> {
    /// Information about the service, for humans
    pub spec: Spec,
    /// Status history of the service
    pub statuses: Buffer,
}

impl<Buffer: StatusBuffer> Watcher<Buffer> {
    #[must_use]
    /// Create a new watcher with an empty history.
    pub const fn new(spec: Spec, buf: Buffer) -> Self {
        Self {
            spec,
            statuses: buf,
        }
    }
}

impl<Buffer: StatusBuffer> Serialize for Watcher<Buffer> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("spec", &self.spec)?;
        map.serialize_entry("statuses", &self.statuses.as_vec())?;
        map.end()
    }
}

impl<'de, Buffer: StatusBuffer> Deserialize<'de> for Watcher<Buffer> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let deser = deserializer.deserialize_map(WatcherVisitor)?;
        let statuses = deser.statuses;
        let statuses = Buffer::from_vec(statuses);
        Ok(Self {
            spec: deser.spec,
            statuses,
        })
    }
}

struct WatcherVisitor;

impl<'de> Visitor<'de> for WatcherVisitor {
    type Value = Watcher<VecBuffer>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a watcher with its spec and statuses")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut spec = None;
        let mut statuses: Option<VecBuffer> = None;
        while let Some(key) = map.next_key()? {
            match key {
                "spec" => {
                    if spec.is_some() {
                        return Err(serde::de::Error::duplicate_field("spec"));
                    }
                    spec = Some(map.next_value()?);
                }
                "statuses" => {
                    if statuses.is_some() {
                        return Err(serde::de::Error::duplicate_field("statuses"));
                    }
                    statuses = Some(map.next_value()?);
                }
                _ => {
                    return Err(serde::de::Error::unknown_field(key, &["spec", "statuses"]));
                }
            }
        }
        let spec = spec.ok_or_else(|| serde::de::Error::missing_field("spec"))?;
        let statuses = statuses.ok_or_else(|| serde::de::Error::missing_field("statuses"))?;
        // TODO: conversion
        Ok(Watcher { spec, statuses })
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
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn from_vec(vec: VecBuffer) -> Self;
    fn as_vec(&self) -> VecBuffer;
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

    fn from_vec(vec: VecBuffer) -> Self {
        vec
    }

    fn as_vec(&self) -> VecBuffer {
        self.clone()
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
            .map(|(time, status)| (*time, status.clone()))
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn from_vec(vec: VecBuffer) -> Self {
        vec.into_iter().collect()
    }

    fn as_vec(&self) -> VecBuffer {
        self.iter()
            .map(|(time, status)| (*time, status.clone()))
            .collect()
    }
}
