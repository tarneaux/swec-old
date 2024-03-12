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
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::BroadcastStream;
use tracing::warn;

use swec_core::{watcher, ApiInfo, ApiMessage};

pub use state::AppState;

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
        |watcher| (StatusCode::OK, Json(Some(watcher.clone()))),
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
        |watcher| (StatusCode::OK, Json(Some(watcher.spec.clone()))),
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
        .update_watcher_spec(&name, spec.clone())
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |()| (StatusCode::OK, Json(Some(spec))),
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
        |watcher| {
            (
                StatusCode::OK,
                Json(Some(watcher.statuses.clone().collect())),
            )
        },
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
        .add_status(&name, status.clone())
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |()| (StatusCode::CREATED, Json(Some(status))),
        )
}

pub async fn get_watcher_ws(
    ws: WebSocketUpgrade,
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let rx = app_state.read().await.get_watcher_receiver(&name);

    rx.map_or_else(
        |_| StatusCode::NOT_FOUND.into_response(), // TODO: Is this how it should be done?
        |rx| ws.on_upgrade(move |socket| handle_ws(socket, rx)),
    )
}

pub async fn handle_ws(socket: WebSocket, rx: tokio::sync::broadcast::Receiver<ApiMessage>) {
    use futures::{SinkExt, StreamExt};
    let (mut tx, _) = socket.split();

    let mut rx = BroadcastStream::new(rx);

    while let Some(msg) = rx.next().await {
        match msg {
            Ok(msg) => {
                let msg = serde_json::to_string(&msg).unwrap();
                tx.send(Message::Text(msg)).await.unwrap();
            }
            Err(e) => {
                warn!(target: "websockets", "Failed to receive message: {e}");
            }
        };
    }
}

mod state {
    use super::StatusRingBuffer;
    use chrono::Local;
    use std::collections::BTreeMap;
    use swec_core::watcher;
    use swec_core::ApiMessage;
    use tracing::{debug, warn};

    pub struct AppState {
        watchers: BTreeMap<
            String,
            (
                watcher::Watcher<StatusRingBuffer>,
                tokio::sync::broadcast::Sender<ApiMessage>,
            ),
        >,
        pub history_len: usize,
    }

    impl AppState {
        pub fn new(
            watchers: BTreeMap<String, watcher::Watcher<StatusRingBuffer>>,
            history_len: usize,
        ) -> Self {
            Self {
                watchers: watchers
                    .into_iter()
                    .map(|(k, v)| (k, (v, tokio::sync::broadcast::channel(1).0)))
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
                (
                    watcher::Watcher::new(watcher_spec, StatusRingBuffer::new(self.history_len)),
                    tokio::sync::broadcast::channel(1).0,
                ),
            );
            Ok(())
        }

        pub fn remove_watcher(
            &mut self,
            name: &str,
        ) -> Result<watcher::Watcher<StatusRingBuffer>, WatcherDoesNotExist> {
            // TODO: do we need to make sure all websockets are closed?
            self.watchers
                .remove(name)
                .map(|(watcher, _)| watcher)
                .ok_or(WatcherDoesNotExist)
        }

        pub fn update_watcher_spec(
            &mut self,
            name: &str,
            watcher_spec: watcher::Spec,
        ) -> Result<(), WatcherDoesNotExist> {
            match self.watchers.get_mut(name) {
                Some(v) => {
                    v.0.spec = watcher_spec.clone();
                    if let Err(e) = v.1.send(ApiMessage::UpdatedSpec(watcher_spec)) {
                        warn!(target: "websockets", "Failed to send updated spec: {e}, ignoring.");
                    }
                }
                None => return Err(WatcherDoesNotExist),
            }

            Ok(())
        }

        pub fn add_status(
            &mut self,
            name: &str,
            status: watcher::Status,
        ) -> Result<(), WatcherDoesNotExist> {
            let time = Local::now();
            match self.watchers.get_mut(name) {
                Some(v) => {
                    v.0.statuses.push((time, status.clone()));
                    if let Err(e) = v.1.send(ApiMessage::AddedStatus(time, status)) {
                        debug!(target: "websockets", "Failed to send added status: {e}, ignoring since this only means there are no websockets open.");
                    }
                    Ok(())
                }
                None => Err(WatcherDoesNotExist),
            }
        }

        pub fn get_watcher(
            &self,
            name: &str,
        ) -> Result<&watcher::Watcher<StatusRingBuffer>, WatcherDoesNotExist> {
            self.watchers
                .get(name)
                .map(|(watcher, _)| watcher)
                .ok_or(WatcherDoesNotExist)
        }

        pub fn get_watchers(&self) -> BTreeMap<String, watcher::Watcher<StatusRingBuffer>> {
            self.watchers
                .iter()
                .map(|(k, (v, _))| (k.clone(), v.clone()))
                .collect()
        }

        pub fn get_watcher_receiver(
            &self,
            name: &str,
        ) -> Result<tokio::sync::broadcast::Receiver<ApiMessage>, WatcherDoesNotExist> {
            self.watchers
                .get(name)
                .ok_or(WatcherDoesNotExist)
                .map(|(_, rx)| rx.subscribe())
        }
    }

    #[derive(Debug)]
    pub struct WatcherAlreadyExists;
    #[derive(Debug)]
    pub struct WatcherDoesNotExist;
}
