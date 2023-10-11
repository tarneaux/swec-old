/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use serde::{Deserialize, Serialize};
use std::time::Duration;

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
