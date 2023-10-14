/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use crate::watchers::Watcher;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub watchers: Vec<Watcher>,
    pub interval: u64,
    pub histsize: usize,
}

impl Config {
    pub fn read(path: &str) -> Result<Self, ConfigReadingError> {
        let file = std::fs::File::open(path).map_err(ConfigReadingError::FileError)?;
        let config: Self = serde_yaml::from_reader(file).map_err(ConfigReadingError::YamlError)?;
        Ok(config)
    }
}

pub enum ConfigReadingError {
    FileError(std::io::Error),
    YamlError(serde_yaml::Error),
}

impl Display for ConfigReadingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileError(e) => write!(f, "Error while reading config file: {e}"),
            Self::YamlError(e) => write!(f, "Error while parsing config file: {e}"),
        }
    }
}
