use crate::{checker, Spec};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub writable: bool,
    pub swec_version: String,
}

/// A message sent by the server to notify the client of an event on a checker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// The checker's spec was updated.
    UpdatedSpec(checker::Spec),
    /// A status was added to the checker.
    AddedStatus(DateTime<Local>, checker::Status),
    /// The checker's initial spec and status.
    Initial(Spec, Option<(DateTime<Local>, checker::Status)>),
    /// The checker was deleted.
    CheckerDeleted,
    /// The server lagged by the given number of messages which were dropped.
    Lagged(u64),
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
            Self::Lagged(n) => write!(f, "Server lagged and dropped {n} messages"),
        }
    }
}
