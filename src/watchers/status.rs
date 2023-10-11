/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::SystemTime;

#[derive(Clone, Serialize, Deserialize)]
pub enum Status {
    Up(Duration),
    Down(DownReason),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum DownReason {
    Timeout,
    WrongContent,
    WrongStatus,
    Unknown,
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
