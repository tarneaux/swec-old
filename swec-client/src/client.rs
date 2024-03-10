use async_trait::async_trait;
use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use swec_core::{ApiInfo, Spec, Status, VecBuffer, Watcher};

use swec_client_derive::api_query;

#[derive(Clone, Debug)]
pub struct ReadOnly {
    base_url: String,
    client: reqwest::Client,
}

impl ReadApi for ReadOnly {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
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

#[derive(Clone, Debug)]
pub struct ReadWrite {
    base_url: String,
    client: reqwest::Client,
}

impl ReadApi for ReadWrite {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}
impl WriteApi for ReadWrite {
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
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

#[async_trait]
pub trait ReadApi {
    fn base_url(&self) -> &str;
    fn client(&self) -> &reqwest::Client;

    async fn get_info(&self) -> Result<ApiInfo, ApiError> {
        api_query!(get, format!("{}/info", self.base_url()), true)
    }

    async fn get_watchers(&self) -> Result<BTreeMap<String, Watcher<VecBuffer>>, ApiError> {
        api_query!(get, format!("{}/watchers", self.base_url()), true)
    }

    async fn get_watcher(&self, name: &str) -> Result<Watcher<VecBuffer>, ApiError> {
        api_query!(get, format!("{}/watchers/{}", self.base_url(), name), true)
    }

    async fn get_watcher_spec(&self, name: &str) -> Result<Spec, ApiError> {
        api_query!(
            get,
            format!("{}/watchers/{}/spec", self.base_url(), name),
            true
        )
    }

    async fn get_watcher_statuses(
        &self,
        name: &str,
    ) -> Result<Vec<(DateTime<Local>, Status)>, ApiError> {
        api_query!(
            get,
            format!("{}/watchers/{}/statuses", self.base_url(), name),
            true
        )
    }

    async fn get_watcher_status(&self, name: &str, n: u32) -> Result<Status, ApiError> {
        api_query!(
            get,
            format!("{}/watchers/{}/statuses/{}", self.base_url(), name, n),
            true
        )
    }
}

#[async_trait]
pub trait WriteApi {
    fn base_url(&self) -> &str;
    fn client(&self) -> &reqwest::Client;

    async fn delete_watcher(&self, name: &str) -> Result<(), ApiError> {
        api_query!(
            delete,
            format!("{}/watchers/{}", self.base_url(), name),
            false
        )
    }
    async fn post_watcher_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
        api_query!(
            post,
            format!("{}/watchers/{}/spec", self.base_url(), name),
            false,
            spec
        )
    }
    async fn put_watcher_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
        api_query!(
            put,
            format!("{}/watchers/{}/spec", self.base_url(), name),
            false,
            spec
        )
    }
    async fn post_watcher_status(&self, name: &str, status: Status) -> Result<(), ApiError> {
        api_query!(
            post,
            format!("{}/watchers/{}/statuses", self.base_url(), name),
            false,
            status
        )
    }
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
