use crate::{checker, Spec};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

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

impl Display for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UpdatedSpec(spec) => write!(f, "Updated spec: {spec}"),
            Self::AddedStatus(time, status) => {
                write!(f, "Added status at {time}: {status}")
            }
            Self::Initial(spec, status) => {
                write!(f, "Initial spec: {spec}")?;
                if let Some((time, status)) = status {
                    write!(f, ", initial status at {time}: {status}")?;
                }
                Ok(())
            }
            Self::CheckerDeleted => write!(f, "Checker deleted"),
        }
    }
}
