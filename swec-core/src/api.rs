use crate::{checker, Spec};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Info {
    pub writable: bool,
    pub swec_version: String,
}

/// A message sent by the server to notify the client of an event on a checker.
/// # Guarantees
/// The server guarantees that the client will receive messages for all updates of a checker,
/// unless there is a lag (See `CheckerMessage::Lagged`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CheckerMessage {
    /// The checker's initial spec and status.
    /// This is the first message received for a checker, and contains the spec and the first
    /// status if it exists.
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
    /// TODO: send a new `CheckerMessage::Initial message inside this one
    Lagged(u64),
}

impl Display for CheckerMessage {
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

impl Message for CheckerMessage {
    fn new_lag(n: u64) -> Self {
        Self::Lagged(n)
    }
}

/// A message sent by the server to notify the client of an event on the list of checkers.
/// Useful for watching all checkers.
/// # Guarantees
/// - The server guarantees that the client will receive messages for all updates of a checker, unless there is a lag (See `GlobalMessage::Lagged`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ListMessage {
    /// The initial list of checkers.
    Initial(BTreeSet<String>),

    /// A new checker was inserted.
    Insert(String),

    /// A checker was replaced with insert().
    InsertReplace(String),

    /// A checker was removed.
    Remove(String),

    /// The server lagged by the given number of messages which were dropped.
    /// This means the guarantee of receiving all updates for the checker is broken, and the client
    /// should consider the list of checkers to be in an unknown state.
    /// TODO: send a new `GlobalMessage::Initial` inside this one
    Lagged(u64),
}

impl Display for ListMessage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Initial(watchers) => write!(f, "Initial watchers: {watchers:?}"),
            Self::Lagged(n) => write!(f, "Server lagged and dropped {n} messages"),
            Self::Insert(w) => write!(f, "Inserted watcher: {w}"),
            Self::InsertReplace(w) => write!(f, "Inserted and replaced watcher: {w}"),
            Self::Remove(w) => write!(f, "Removed watcher: {w}"),
        }
    }
}

impl Message for ListMessage {
    fn new_lag(n: u64) -> Self {
        Self::Lagged(n)
    }
}

pub trait Message: Clone + Send + Serialize {
    fn new_lag(n: u64) -> Self;
}
