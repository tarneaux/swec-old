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
    /// The checker's initial spec and status.
    /// This is the first message received for a checker, and contains the spec and the first
    /// status if it exists. Any changes from this point should be guaranteed to be sent to the
    /// client receiving this message (e.g. through a websocket).
    Initial(Spec, Option<(DateTime<Local>, checker::Status)>),

    /// The checker's spec was updated.
    UpdatedSpec(checker::Spec),

    /// A status was added to the checker.
    AddedStatus(DateTime<Local>, checker::Status),

    /// The checker was dropped by the server.
    /// This should be the last message received for the checker; after this, the server will
    /// either shut down or the watcher will be removed, both of which will result in the
    /// websocket being closed.
    CheckerDropped,

    /// The server lagged by the given number of messages which were dropped.
    /// This means the guarantee of receiving all updates for the checker is broken, and the client
    /// should consider the checker to be in an unknown state.
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
            Self::CheckerDropped => write!(f, "Checker dropped by server"),
            Self::Lagged(n) => write!(f, "Server lagged and dropped {n} messages"),
        }
    }
}
