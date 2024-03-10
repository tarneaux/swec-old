use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub writable: bool,
    pub swec_version: String,
}
