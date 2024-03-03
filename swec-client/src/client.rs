use chrono::{DateTime, Local};
use std::collections::BTreeMap;
use swec_core::*;

use std::future::Future;
use swec_client_derive::*;

#[derive(Clone, Debug, ReadApi)]
pub struct ReadOnlyClient {
    base_url: String,
    client: reqwest::Client,
}

impl ReadOnlyClient {
    pub fn new(base_url: String) -> Self {
        ReadOnlyClient {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Clone, Debug, ReadApi, WriteApi)]
pub struct ReadWriteClient {
    base_url: String,
    client: reqwest::Client,
}

impl ReadWriteClient {
    pub fn new(base_url: String) -> Self {
        ReadWriteClient {
            base_url,
            client: reqwest::Client::new(),
        }
    }
}

pub trait ReadApi {
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
        ApiError::Reqwest(e)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Serde(e)
    }
}
