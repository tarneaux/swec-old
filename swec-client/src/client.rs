use async_trait::async_trait;
use chrono::{DateTime, Local};
use futures_util::StreamExt;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use swec_core::{ApiInfo, ApiMessage, Spec, Status, VecBuffer, Checker};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::warn;

use swec_client_derive::api_query;

#[derive(Clone, Debug)]
pub struct ReadOnly {
    base_url: String,
    ws_base_url: String,
    client: reqwest::Client,
}

impl Api for ReadOnly {}
impl ReadApi for ReadOnly {}

impl ApiPrivate for ReadOnly {
    fn new_with_urls(base_url: String, ws_base_url: String) -> Self {
        Self {
            base_url,
            ws_base_url,
            client: reqwest::Client::new(),
        }
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn ws_base_url(&self) -> &str {
        &self.ws_base_url
    }
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[derive(Clone, Debug)]
pub struct ReadWrite {
    base_url: String,
    ws_base_url: String,
    client: reqwest::Client,
}

impl Api for ReadWrite {}
impl ReadApi for ReadWrite {}
impl WriteApi for ReadWrite {}

impl ApiPrivate for ReadWrite {
    fn new_with_urls(base_url: String, ws_base_url: String) -> Self {
        Self {
            base_url,
            ws_base_url,
            client: reqwest::Client::new(),
        }
    }
    fn base_url(&self) -> &str {
        &self.base_url
    }
    fn ws_base_url(&self) -> &str {
        &self.ws_base_url
    }
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

pub trait Api: ApiPrivate {
    /// Create a new client.
    /// `base_url` should be the URL of the API server, for example `http://localhost:8081/api/v1`.
    /// # Errors
    /// Returns `UrlFormatError` if the base URL is not a valid URL (i.e. does not start with `http://` or `https://`).
    fn new(base_url: String) -> Result<Self, UrlFormatError>
    where
        Self: Sized,
    {
        if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
            return Err(UrlFormatError(base_url));
        }
        let base_url: String = base_url.trim_end_matches('/').to_string();
        let ws_base_url = base_url.replacen("http", "ws", 1);
        Ok(Self::new_with_urls(base_url, ws_base_url))
    }
}

/// Private methods for the API.
/// Should not be used directly; use the public methods from `Api`, `ReadApi`, and `WriteApi` instead.
pub trait ApiPrivate {
    fn new_with_urls(base_url: String, ws_base_url: String) -> Self
    where
        Self: Sized;
    fn base_url(&self) -> &str;
    fn ws_base_url(&self) -> &str;
    fn client(&self) -> &reqwest::Client;
}

#[async_trait]
pub trait ReadApi: Api {
    async fn get_info(&self) -> Result<ApiInfo, ApiError> {
        api_query!(get, format!("{}/info", self.base_url()), true)
    }

    async fn get_checkers(&self) -> Result<BTreeMap<String, Checker<VecBuffer>>, ApiError> {
        api_query!(get, format!("{}/checkers", self.base_url()), true)
    }

    async fn get_checker(&self, name: &str) -> Result<Checker<VecBuffer>, ApiError> {
        api_query!(get, format!("{}/checkers/{}", self.base_url(), name), true)
    }

    async fn get_checker_spec(&self, name: &str) -> Result<Spec, ApiError> {
        api_query!(
            get,
            format!("{}/checkers/{}/spec", self.base_url(), name),
            true
        )
    }

    async fn get_checker_statuses(
        &self,
        name: &str,
    ) -> Result<Vec<(DateTime<Local>, Status)>, ApiError> {
        api_query!(
            get,
            format!("{}/checkers/{}/statuses", self.base_url(), name),
            true
        )
    }

    async fn get_checker_status(&self, name: &str, n: u32) -> Result<Status, ApiError> {
        api_query!(
            get,
            format!("{}/checkers/{}/statuses/{}", self.base_url(), name, n),
            true
        )
    }

    async fn watch_checker(
        &self,
        name: &str,
        channel: Sender<ApiMessage>,
    ) -> Result<JoinHandle<()>, WsError> {
        let (ws_stream, _) =
            connect_async(format!("{}/checkers/{}/watch", self.ws_base_url(), name)).await?;
        let (_, mut read) = ws_stream.split();

        // Spawn a new task that will forward messages from the websocket to the channel
        Ok(tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                async fn f(
                    msg: Result<Message, tokio_tungstenite::tungstenite::Error>,
                    channel: &Sender<ApiMessage>,
                ) -> Result<(), Box<dyn Error>> {
                    let msg = msg?;
                    let msg_text = msg.to_text()?;
                    let status = serde_json::from_str(msg_text)?;
                    channel.send(status).await?;
                    Ok(())
                }

                if let Err(e) = f(msg, &channel).await {
                    // TODO: What are the possible errors here? Should we exit the task for some of them?
                    warn!("Error reading from websocket: {e}, ignoring");
                }
            }
        }))
    }
}

#[async_trait]
pub trait WriteApi: Api {
    async fn delete_checker(&self, name: &str) -> Result<(), ApiError> {
        api_query!(
            delete,
            format!("{}/checkers/{}", self.base_url(), name),
            false
        )
    }
    async fn post_checker_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
        api_query!(
            post,
            format!("{}/checkers/{}/spec", self.base_url(), name),
            false,
            spec
        )
    }
    async fn put_checker_spec(&self, name: &str, spec: Spec) -> Result<(), ApiError> {
        api_query!(
            put,
            format!("{}/checkers/{}/spec", self.base_url(), name),
            false,
            spec
        )
    }
    async fn post_checker_status(&self, name: &str, status: Status) -> Result<(), ApiError> {
        api_query!(
            post,
            format!("{}/checkers/{}/statuses", self.base_url(), name),
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

#[derive(Debug)]
pub enum WsError {
    Tungstenite(tokio_tungstenite::tungstenite::Error),
}

impl From<tokio_tungstenite::tungstenite::Error> for WsError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::Tungstenite(e)
    }
}

impl std::error::Error for ApiError {}

#[derive(Debug)]
pub struct UrlFormatError(String);

impl Display for UrlFormatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Invalid URL format: {}. Urls should start with http:// or https://",
            self.0
        )
    }
}
