use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use swec_core::{ApiInfo, Spec, Status, VecBuffer, Watcher};

use std::future::Future;
use swec_client_derive::{api_query, ReadApi, WriteApi};

#[derive(Clone, Debug, ReadApi)]
pub struct ReadOnly {
    base_url: String,
    client: reqwest::Client,
}

impl ReadOnly {
    #[must_use]
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Clone, Debug, ReadApi, WriteApi)]
pub struct ReadWrite {
    base_url: String,
    client: reqwest::Client,
}

impl ReadWrite {
    #[must_use]
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

pub trait ReadApi {
    fn get_info(&self) -> impl Future<Output = Result<ApiInfo, ApiError>> + Send;
    fn get_watchers(
        &self,
    ) -> impl Future<Output = Result<BTreeMap<String, Watcher<VecBuffer>>, ApiError>> + Send;
    fn get_watcher(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Watcher<VecBuffer>, ApiError>> + Send;
    fn get_watcher_spec(&self, name: &str) -> impl Future<Output = Result<Spec, ApiError>> + Send;
    fn get_watcher_statuses(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Vec<(DateTime<Local>, Status)>, ApiError>> + Send;
    fn get_watcher_status(
        &self,
        name: &str,
        n: u32,
    ) -> impl Future<Output = Result<Status, ApiError>> + Send;
}

pub trait WriteApi {
    fn delete_watcher(&self, name: &str) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn post_watcher_spec(
        &self,
        name: &str,
        spec: Spec,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn put_watcher_spec(
        &self,
        name: &str,
        spec: Spec,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
    fn post_watcher_status(
        &self,
        name: &str,
        status: Status,
    ) -> impl Future<Output = Result<(), ApiError>> + Send;
}

#[derive(Debug)]
pub enum ApiError {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        Self::Reqwest(e)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e)
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reqwest(e) => write!(f, "Reqwest error: {e}"),
            Self::Serde(e) => write!(f, "Serde error: {e}"),
        }
    }
}

impl std::error::Error for ApiError {}