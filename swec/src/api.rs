use crate::StatusRingBuffer;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json,
};
use chrono::{DateTime, Local};
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use std::collections::BTreeMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use tracing::{info, warn};

use swec_core::{watcher, ApiInfo, ApiMessage};

pub use watcher_with_sender::WatcherWithSender;

// The read-only API.
pub fn read_only_router() -> axum::Router<(ApiInfo, Arc<RwLock<AppState>>)> {
    axum::Router::new()
        .route("/info", get(get_api_info))
        .route("/watchers", get(get_watchers))
        .route("/watchers/:name", get(get_watcher))
        .route("/watchers/:name/spec", get(get_watcher_spec))
        .route("/watchers/:name/statuses", get(get_watcher_statuses))
        .route("/watchers/:name/statuses/:index", get(get_watcher_status))
        .route("/watchers/:name/watch", get(get_watcher_ws))
}

// The read-write API.
pub fn read_write_router() -> axum::Router<(ApiInfo, Arc<RwLock<AppState>>)> {
    read_only_router()
        .route("/watchers/:name", delete(delete_watcher))
        .route("/watchers/:name/spec", post(post_watcher_spec))
        .route("/watchers/:name/spec", put(put_watcher_spec))
        .route("/watchers/:name/statuses", post(post_watcher_status))
}

pub async fn get_api_info(
    State((api_info, _)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> Json<ApiInfo> {
    Json(api_info)
}

pub async fn get_watchers(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> (
    StatusCode,
    Json<BTreeMap<String, watcher::Watcher<StatusRingBuffer>>>,
) {
    let watchers = app_state.read().await.get_watchers();
    (StatusCode::OK, Json(watchers))
}

pub async fn get_watcher(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Watcher<StatusRingBuffer>>>) {
    app_state.read().await.get_watcher(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher))),
    )
}

pub async fn delete_watcher(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Watcher<StatusRingBuffer>>>) {
    app_state.write().await.remove_watcher(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher))),
    )
}

pub async fn get_watcher_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Spec>>) {
    app_state.read().await.get_watcher(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher.spec))),
    )
}

pub async fn post_watcher_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(spec): Json<watcher::Spec>,
) -> (StatusCode, Json<Option<watcher::Spec>>) {
    app_state
        .write()
        .await
        .add_watcher(name, spec.clone())
        .map_or_else(
            |_| (StatusCode::CONFLICT, Json(None)),
            |()| (StatusCode::CREATED, Json(Some(spec))),
        )
}

pub async fn put_watcher_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(spec): Json<watcher::Spec>,
) -> (StatusCode, Json<Option<watcher::Spec>>) {
    app_state
        .write()
        .await
        .get_watcher_with_sender_mut(&name)
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |watcher| {
                watcher.update_spec(spec.clone());
                (StatusCode::OK, Json(Some(spec)))
            },
        )
}

pub async fn get_watcher_statuses(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (
    StatusCode,
    Json<Option<Vec<(DateTime<Local>, watcher::Status)>>>,
) {
    app_state.read().await.get_watcher(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher.statuses.collect()))),
    )
}

pub async fn get_watcher_status(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path((name, index)): Path<(String, usize)>,
) -> (StatusCode, Json<Option<(DateTime<Local>, watcher::Status)>>) {
    app_state.read().await.get_watcher(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |watcher| {
            watcher.statuses.iter().rev().nth(index).map_or_else(
                || (StatusCode::NOT_FOUND, Json(None)),
                |status| (StatusCode::OK, Json(Some(status.clone()))),
            )
        },
    )
}

pub async fn post_watcher_status(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(status): Json<watcher::Status>,
) -> (StatusCode, Json<Option<watcher::Status>>) {
    app_state
        .write()
        .await
        .get_watcher_with_sender_mut(&name)
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |w| {
                w.add_status(status.clone());
                (StatusCode::CREATED, Json(Some(status)))
            },
        )
}

pub async fn get_watcher_ws(
    ws: WebSocketUpgrade,
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // The `Initial` message we send is meant to avoid race conditions where the client would first
    // ask for the current state and then subscribe to updates. This way, the client can just
    // subscribe and get the current state in one go.
    // The fact that we subscribe and create the `Initial` message in the same atomic operation is
    // important to make sure there is no race condition here.
    let res = app_state
        .read()
        .await
        .get_watcher_with_sender(&name)
        .map(|w| {
            (
                w.subscribe(),
                ApiMessage::Initial(
                    w.watcher().spec.clone(),
                    w.watcher().statuses.iter().next_back().cloned(),
                ),
            )
        });

    if let Ok((rx, initial_message)) = res {
        ws.on_upgrade(move |socket| handle_ws(socket, rx, initial_message))
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn handle_ws(
    socket: WebSocket,
    broadcast_rx: tokio::sync::broadcast::Receiver<ApiMessage>,
    initial_message: ApiMessage,
) {
    async fn send(
        tx: &mut SplitSink<WebSocket, Message>,
        msg: ApiMessage,
    ) -> Result<(), Box<dyn Error>> {
        let msg = serde_json::to_string(&msg)?;
        tx.send(Message::Text(msg)).await?;
        Ok(())
    }
    let (mut socket_tx, mut socket_rx) = socket.split();

    let mut broadcast_rx = BroadcastStream::new(broadcast_rx);

    send(&mut socket_tx, initial_message)
        .await
        .unwrap_or_else(|e| {
            warn!(target: "websockets", "Failed to send initial message: {e}");
        });

    let handle = tokio::spawn(async move {
        while let Some(msg) = broadcast_rx.next().await {
            match msg {
                Ok(msg) => {
                    if let Err(e) = send(&mut socket_tx, msg).await {
                        warn!(target: "websockets", "Failed to send websocket message: {e}");
                        break;
                    }
                }
                Err(e) => {
                    warn!(target: "websockets", "Failed to receive websocket message: {e}");
                }
            };
        }
        // Needed because we use socket_rx below, preventing the socket from being dropped
        socket_tx.close().await.unwrap_or_else(|e| {
            warn!(target: "websockets", "Failed to close websocket: {e}");
        });
    });

    while socket_rx.next().await.is_some() {}
    handle.abort();
    info!(target: "websockets", "Websocket closed");
}

pub struct AppState {
    watchers: BTreeMap<String, WatcherWithSender>,
    history_len: usize,
}

impl AppState {
    pub fn new(
        watchers: BTreeMap<String, watcher::Watcher<StatusRingBuffer>>,
        history_len: usize,
    ) -> Self {
        Self {
            watchers: watchers
                .into_iter()
                .map(|(k, v)| (k, WatcherWithSender::new(v)))
                .collect(),
            history_len,
        }
    }

    pub fn add_watcher(
        &mut self,
        name: String,
        watcher_spec: watcher::Spec,
    ) -> Result<(), WatcherAlreadyExists> {
        if self.watchers.contains_key(&name) {
            return Err(WatcherAlreadyExists);
        }
        self.watchers.insert(
            name,
            WatcherWithSender::new(watcher::Watcher::new(
                watcher_spec,
                StatusRingBuffer::new(self.history_len),
            )),
        );
        Ok(())
    }

    pub fn remove_watcher(
        &mut self,
        name: &str,
    ) -> Result<watcher::Watcher<StatusRingBuffer>, WatcherDoesNotExist> {
        // TODO: do we need to make sure all websockets are closed?
        //       => method in WatcherWithSender to close all websockets gracefully with a message
        self.watchers
            .remove(name)
            .map(|w| w.watcher().clone())
            .ok_or(WatcherDoesNotExist)
    }

    pub fn get_watcher(
        &self,
        name: &str,
    ) -> Result<watcher::Watcher<StatusRingBuffer>, WatcherDoesNotExist> {
        self.get_watcher_with_sender(name)
            .map(|w| w.watcher().clone())
    }

    pub fn get_watcher_with_sender(
        &self,
        name: &str,
    ) -> Result<&WatcherWithSender, WatcherDoesNotExist> {
        self.watchers.get(name).ok_or(WatcherDoesNotExist)
    }

    pub fn get_watcher_with_sender_mut(
        &mut self,
        name: &str,
    ) -> Result<&mut WatcherWithSender, WatcherDoesNotExist> {
        self.watchers.get_mut(name).ok_or(WatcherDoesNotExist)
    }

    pub fn get_watchers(&self) -> BTreeMap<String, watcher::Watcher<StatusRingBuffer>> {
        self.watchers
            .iter()
            .map(|(k, v)| (k.clone(), v.watcher().clone()))
            .collect()
    }

    pub fn watchers_to_json(&self) -> Result<String, serde_json::Error> {
        let watchers: BTreeMap<String, watcher::Watcher<StatusRingBuffer>> = self
            .watchers
            .iter()
            .map(|(k, v)| (k.clone(), v.watcher().clone()))
            .collect();
        serde_json::to_string(&watchers)
    }
}

#[derive(Debug)]
pub struct WatcherAlreadyExists;
#[derive(Debug)]
pub struct WatcherDoesNotExist;

mod watcher_with_sender {
    use super::StatusRingBuffer;
    use chrono::Local;
    use swec_core::watcher;
    use swec_core::ApiMessage;
    use tracing::{debug, warn};

    #[derive(Debug)]
    /// Encapsulates a `watcher::Watcher` with a `tokio::sync::broadcast::Sender` to send updates
    /// to subscribers. This needs to be in a separate module for the privacy of the inner fields
    /// (to that we don't modify a watcher without sending an update).
    pub struct WatcherWithSender {
        watcher: watcher::Watcher<StatusRingBuffer>,
        sender: tokio::sync::broadcast::Sender<ApiMessage>,
    }

    impl WatcherWithSender {
        pub fn new(watcher: watcher::Watcher<StatusRingBuffer>) -> Self {
            let (sender, _) = tokio::sync::broadcast::channel(1); // TODO: what should the capacity be? allow changing it?
            Self { watcher, sender }
        }

        pub const fn watcher(&self) -> &watcher::Watcher<StatusRingBuffer> {
            &self.watcher
        }

        pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ApiMessage> {
            self.sender.subscribe()
        }

        pub fn update_spec(&mut self, spec: watcher::Spec) {
            self.watcher.spec = spec.clone();
            if let Err(e) = self.sender.send(ApiMessage::UpdatedSpec(spec)) {
                warn!(target: "websockets", "Failed to send updated spec: {e}, ignoring.");
            }
        }

        pub fn add_status(&mut self, status: watcher::Status) {
            let time = Local::now();
            self.watcher.statuses.push((time, status.clone()));
            if let Err(e) = self.sender.send(ApiMessage::AddedStatus(time, status)) {
                debug!(target: "websockets", "Failed to send added status: {e}, ignoring.");
            }
        }
    }

    impl Drop for WatcherWithSender {
        fn drop(&mut self) {
            if let Err(e) = self.sender.send(ApiMessage::WatcherDeleted) {
                warn!(target: "websockets", "Failed to send WatcherDropped: {e}, ignoring.");
            }
        }
    }
}
