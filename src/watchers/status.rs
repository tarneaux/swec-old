/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Status {
    Up(Duration),
    Down(DownReason),
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Up(duration) => write!(f, "Up: took {}", duration.as_secs()),
            Self::Down(reason) => write!(f, "Down: {reason}"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DownReason {
    Timeout,
    WrongContent,
    WrongStatus,
    Unknown,
}

impl Display for DownReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Timeout"),
            Self::WrongContent => write!(f, "Wrong content"),
            Self::WrongStatus => write!(f, "Wrong status"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TimeStampedStatus {
    pub status: Status,
    pub time: SystemTime,
}

impl TimeStampedStatus {
    pub fn new_now(status: Status) -> Self {
        Self {
            status,
            time: SystemTime::now(),
        }
    }
}
