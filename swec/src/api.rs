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
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use tracing::{info, warn};

use swec_core::{checker, ApiInfo, ApiMessage, CheckerMessage, ListMessage};

pub use checker_with_sender::CheckerWithSender;

use self::btreemap_with_sender::BTreeMapWithSender;

// The read-only API.
pub fn read_only_router() -> axum::Router<(ApiInfo, Arc<RwLock<AppState>>)> {
    axum::Router::new()
        .route("/info", get(get_api_info))
        .route("/checkers", get(get_checkers))
        .route("/checker_names", get(get_checker_names))
        .route("/watch", get(get_global_ws))
        .route("/checkers/:name", get(get_checker))
        .route("/checkers/:name/spec", get(get_checker_spec))
        .route("/checkers/:name/statuses", get(get_checker_statuses))
        .route("/checkers/:name/statuses/:index", get(get_checker_status))
        .route("/checkers/:name/watch", get(get_checker_ws))
}

// The read-write API.
pub fn read_write_router() -> axum::Router<(ApiInfo, Arc<RwLock<AppState>>)> {
    read_only_router()
        .route("/checkers/:name", delete(delete_checker))
        .route("/checkers/:name/spec", post(post_checker_spec))
        .route("/checkers/:name/spec", put(put_checker_spec))
        .route("/checkers/:name/statuses", post(post_checker_status))
}

pub async fn get_api_info(
    State((api_info, _)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> Json<ApiInfo> {
    Json(api_info)
}

pub async fn get_checkers(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> (
    StatusCode,
    Json<BTreeMap<String, checker::Checker<StatusRingBuffer>>>,
) {
    let checkers = app_state.read().await.get_checkers();
    (StatusCode::OK, Json(checkers))
}

pub async fn get_checker_names(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> Json<Vec<String>> {
    Json(app_state.read().await.checkers.keys().cloned().collect())
}

pub async fn get_checker(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<checker::Checker<StatusRingBuffer>>>) {
    app_state.read().await.get_checker(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |checker| (StatusCode::OK, Json(Some(checker))),
    )
}

pub async fn delete_checker(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<checker::Checker<StatusRingBuffer>>>) {
    app_state.write().await.remove_checker(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |checker| (StatusCode::OK, Json(Some(checker))),
    )
}

pub async fn get_checker_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<checker::Spec>>) {
    app_state.read().await.get_checker(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |checker| (StatusCode::OK, Json(Some(checker.spec))),
    )
}

pub async fn post_checker_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(spec): Json<checker::Spec>,
) -> (StatusCode, Json<Option<checker::Spec>>) {
    app_state
        .write()
        .await
        .add_checker(name, spec.clone())
        .map_or_else(
            |_| (StatusCode::CONFLICT, Json(None)),
            |()| (StatusCode::CREATED, Json(Some(spec))),
        )
}

pub async fn put_checker_spec(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(spec): Json<checker::Spec>,
) -> (StatusCode, Json<Option<checker::Spec>>) {
    app_state
        .write()
        .await
        .get_checker_with_sender_mut(&name)
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |checker| {
                checker.update_spec(spec.clone());
                (StatusCode::OK, Json(Some(spec)))
            },
        )
}

pub async fn get_checker_statuses(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
) -> (
    StatusCode,
    Json<Option<Vec<(DateTime<Local>, checker::Status)>>>,
) {
    app_state.read().await.get_checker(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |checker| (StatusCode::OK, Json(Some(checker.statuses.collect()))),
    )
}

pub async fn get_checker_status(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path((name, index)): Path<(String, usize)>,
) -> (StatusCode, Json<Option<(DateTime<Local>, checker::Status)>>) {
    app_state.read().await.get_checker(&name).map_or_else(
        |_| (StatusCode::NOT_FOUND, Json(None)),
        |checker| {
            checker.statuses.iter().rev().nth(index).map_or_else(
                || (StatusCode::NOT_FOUND, Json(None)),
                |status| (StatusCode::OK, Json(Some(status.clone()))),
            )
        },
    )
}

pub async fn post_checker_status(
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
    Path(name): Path<String>,
    Json(status): Json<checker::Status>,
) -> (StatusCode, Json<Option<checker::Status>>) {
    app_state
        .write()
        .await
        .get_checker_with_sender_mut(&name)
        .map_or_else(
            |_| (StatusCode::NOT_FOUND, Json(None)),
            |w| {
                w.add_status(status.clone());
                (StatusCode::CREATED, Json(Some(status)))
            },
        )
}

pub async fn get_checker_ws(
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
        .get_checker_with_sender(&name)
        .map(|w| {
            (
                w.subscribe(),
                CheckerMessage::Initial(
                    w.checker().spec.clone(),
                    w.checker().statuses.iter().next_back().cloned(),
                ),
            )
        });

    if let Ok((rx, initial_message)) = res {
        ws.on_upgrade(move |socket| handle_ws(socket, rx, initial_message))
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn get_global_ws(
    ws: WebSocketUpgrade,
    State((_, app_state)): State<(ApiInfo, Arc<RwLock<AppState>>)>,
) -> impl IntoResponse {
    let (rx, initial_checkers): (
        tokio::sync::broadcast::Receiver<ListMessage>,
        BTreeSet<String>,
    ) = {
        let c = &app_state.read().await.checkers;
        (c.subscribe(), c.keys().cloned().collect())
    };

    let initial_message = ListMessage::Initial(initial_checkers);

    ws.on_upgrade(move |socket| handle_ws(socket, rx, initial_message))
}

pub async fn handle_ws<M: ApiMessage + 'static>(
    socket: WebSocket,
    broadcast_rx: tokio::sync::broadcast::Receiver<M>,
    initial_message: M,
) {
    async fn send<M: serde::Serialize + Send>(
        tx: &mut SplitSink<WebSocket, Message>,
        msg: M,
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
                Err(e) => match e {
                    BroadcastStreamRecvError::Lagged(n) => {
                        warn!(target: "websockets", "Lagged and skipped {n} messages. Informing client.");
                        if let Err(e) = send(&mut socket_tx, CheckerMessage::Lagged(n)).await {
                            warn!(target: "websockets", "Failed to send Lagged message: {e}");
                            break;
                        }
                    }
                },
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
    checkers: BTreeMapWithSender<CheckerWithSender>,
    history_len: usize,
}

impl AppState {
    pub fn new(
        checkers: BTreeMap<String, checker::Checker<StatusRingBuffer>>,
        history_len: usize,
    ) -> Self {
        Self {
            checkers: checkers
                .into_iter()
                .map(|(k, v)| (k, CheckerWithSender::new(v)))
                .collect::<BTreeMap<String, CheckerWithSender>>()
                .into(),
            history_len,
        }
    }

    pub fn add_checker(
        &mut self,
        name: String,
        checker_spec: checker::Spec,
    ) -> Result<(), CheckerAlreadyExists> {
        if self.checkers.inner().contains_key(&name) {
            return Err(CheckerAlreadyExists);
        }
        self.checkers.insert(
            name,
            CheckerWithSender::new(checker::Checker::new(
                checker_spec,
                StatusRingBuffer::new(self.history_len),
            )),
        );
        Ok(())
    }

    pub fn remove_checker(
        &mut self,
        name: &str,
    ) -> Result<checker::Checker<StatusRingBuffer>, CheckerDoesNotExist> {
        // The websockets will be gracefully closed when the CheckerWithSender is dropped.
        self.checkers
            .remove(name)
            .map(|w| w.checker().clone())
            .ok_or(CheckerDoesNotExist)
    }

    pub fn get_checker(
        &self,
        name: &str,
    ) -> Result<checker::Checker<StatusRingBuffer>, CheckerDoesNotExist> {
        self.get_checker_with_sender(name)
            .map(|w| w.checker().clone())
    }

    pub fn get_checker_with_sender(
        &self,
        name: &str,
    ) -> Result<&CheckerWithSender, CheckerDoesNotExist> {
        self.checkers.inner().get(name).ok_or(CheckerDoesNotExist)
    }

    pub fn get_checker_with_sender_mut(
        &mut self,
        name: &str,
    ) -> Result<&mut CheckerWithSender, CheckerDoesNotExist> {
        self.checkers.get_mut(name).ok_or(CheckerDoesNotExist)
    }

    pub fn get_checkers(&self) -> BTreeMap<String, checker::Checker<StatusRingBuffer>> {
        self.checkers
            .inner()
            .iter()
            .map(|(k, v)| (k.clone(), v.checker().clone()))
            .collect()
    }

    pub fn checkers_to_json(&self) -> Result<String, serde_json::Error> {
        let checkers: BTreeMap<String, checker::Checker<StatusRingBuffer>> = self
            .checkers
            .inner()
            .iter()
            .map(|(k, v)| (k.clone(), v.checker().clone()))
            .collect();
        serde_json::to_string(&checkers)
    }
}

#[derive(Debug)]
pub struct CheckerAlreadyExists;
#[derive(Debug)]
pub struct CheckerDoesNotExist;

mod btreemap_with_sender {
    use std::collections::{btree_map, BTreeMap};
    use swec_core::ListMessage;
    use tracing::warn;

    #[derive(Debug)]
    pub struct BTreeMapWithSender<T> {
        btreemap: BTreeMap<String, T>,
        sender: tokio::sync::broadcast::Sender<ListMessage>,
    }

    impl<T> BTreeMapWithSender<T> {
        #[must_use]
        pub fn new() -> Self {
            Self {
                btreemap: BTreeMap::new(),
                sender: tokio::sync::broadcast::channel(16).0,
            }
        }

        pub fn keys(&self) -> btree_map::Keys<'_, std::string::String, T> {
            self.btreemap.keys()
        }

        pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<ListMessage> {
            self.sender.subscribe()
        }

        pub const fn inner(&self) -> &BTreeMap<String, T> {
            &self.btreemap
        }

        pub fn get_mut(&mut self, key: &str) -> Option<&mut T> {
            self.btreemap.get_mut(key)
        }

        pub fn insert(&mut self, key: String, value: T) -> Option<T> {
            let r = self.btreemap.insert(key.clone(), value);
            let msg = match r {
                Some(_) => ListMessage::InsertReplace(key),
                None => ListMessage::Insert(key),
            };
            if let Err(e) = self.sender.send(msg) {
                warn!(target: "websockets", "Failed to send msg: {e}, ignoring.");
            }
            r
        }

        pub fn remove(&mut self, key: &str) -> Option<T> {
            match self.btreemap.remove(key) {
                Some(v) => {
                    if let Err(e) = self.sender.send(ListMessage::Remove(key.to_string())) {
                        warn!(target: "websockets", "Failed to send Remove: {e}, ignoring.");
                    }
                    Some(v)
                }
                None => None,
            }
        }
    }

    impl<T> From<BTreeMap<String, T>> for BTreeMapWithSender<T> {
        fn from(btreemap: BTreeMap<String, T>) -> Self {
            Self {
                btreemap,
                sender: tokio::sync::broadcast::channel(16).0,
            }
        }
    }
}

mod checker_with_sender {
    use super::StatusRingBuffer;
    use chrono::Local;
    use swec_core::checker;
    use swec_core::CheckerMessage;
    use tracing::{debug, warn};

    #[derive(Debug)]
    /// Encapsulates a `checker::Checker` with a `tokio::sync::broadcast::Sender` to send updates
    /// to subscribers. This needs to be in a separate module for the privacy of the inner fields
    /// (so that we don't modify a checker without sending an update).
    pub struct CheckerWithSender {
        checker: checker::Checker<StatusRingBuffer>,
        sender: tokio::sync::broadcast::Sender<CheckerMessage>,
    }

    impl CheckerWithSender {
        pub fn new(checker: checker::Checker<StatusRingBuffer>) -> Self {
            let (sender, _) = tokio::sync::broadcast::channel(16);
            Self { checker, sender }
        }

        pub const fn checker(&self) -> &checker::Checker<StatusRingBuffer> {
            &self.checker
        }

        pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CheckerMessage> {
            self.sender.subscribe()
        }

        pub fn update_spec(&mut self, spec: checker::Spec) {
            self.checker.spec = spec.clone();
            if let Err(e) = self.sender.send(CheckerMessage::UpdatedSpec(spec)) {
                warn!(target: "websockets", "Failed to send updated spec: {e}, ignoring.");
            }
        }

        pub fn add_status(&mut self, status: checker::Status) {
            let time = Local::now();
            self.checker.statuses.push((time, status.clone()));
            if let Err(e) = self.sender.send(CheckerMessage::AddedStatus(time, status)) {
                debug!(target: "websockets", "Failed to send added status: {e}, ignoring.");
            }
        }
    }

    impl Drop for CheckerWithSender {
        fn drop(&mut self) {
            if let Err(e) = self.sender.send(CheckerMessage::CheckerDropped) {
                warn!(target: "websockets", "Failed to send CheckerDropped: {e}, ignoring.");
            }
        }
    }
}
