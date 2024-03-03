use crate::StatusRingBuffer;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json,
};
use chrono::{DateTime, Local};
use color_eyre::eyre::{eyre, Result};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use swec_core::watcher;

// The read-only API.
pub fn read_only_router() -> axum::Router<Arc<RwLock<AppState>>> {
    axum::Router::new()
        .route("/watchers", get(get_watchers))
        .route("/watchers/:name", get(get_watcher))
        .route("/watchers/:name/spec", get(get_watcher_spec))
        .route("/watchers/:name/statuses", get(get_watcher_statuses))
        .route("/watchers/:name/statuses/:index", get(get_watcher_status))
}

// The read-write API.
pub fn read_write_router() -> axum::Router<Arc<RwLock<AppState>>> {
    read_only_router()
        .route("/watchers/:name", delete(delete_watcher))
        .route("/watchers/:name/spec", post(post_watcher_spec))
        .route("/watchers/:name/spec", put(put_watcher_spec))
        .route("/watchers/:name/statuses", post(post_watcher_status))
}

pub struct AppState {
    pub watchers: BTreeMap<String, watcher::Watcher<StatusRingBuffer>>,
    pub history_len: usize,
}

impl AppState {
    fn add_watcher(&mut self, name: String, watcher_spec: watcher::Spec) -> Result<()> {
        if self.watchers.contains_key(&name) {
            return Err(eyre!("Watcher already exists"));
        }
        self.watchers.insert(
            name,
            watcher::Watcher::new(watcher_spec, StatusRingBuffer::new(self.history_len)),
        );
        Ok(())
    }
}

pub async fn get_watchers(
    State(app_state): State<Arc<RwLock<AppState>>>,
) -> (
    StatusCode,
    Json<BTreeMap<String, watcher::Watcher<StatusRingBuffer>>>,
) {
    let watchers = &app_state.read().await.watchers;
    (StatusCode::OK, Json(watchers.clone()))
}

pub async fn get_watcher(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Watcher<StatusRingBuffer>>>) {
    app_state.read().await.watchers.get(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher.clone()))),
    )
}

pub async fn delete_watcher(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Watcher<StatusRingBuffer>>>) {
    app_state.write().await.watchers.remove(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher))),
    )
}

pub async fn get_watcher_spec(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
) -> (StatusCode, Json<Option<watcher::Spec>>) {
    app_state.read().await.watchers.get(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| (StatusCode::OK, Json(Some(watcher.spec.clone()))),
    )
}

pub async fn post_watcher_spec(
    State(app_state): State<Arc<RwLock<AppState>>>,
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
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
    Json(spec): Json<watcher::Spec>,
) -> (StatusCode, Json<Option<watcher::Spec>>) {
    app_state.write().await.watchers.get_mut(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| {
            watcher.spec = spec;
            (StatusCode::OK, Json(Some(watcher.spec.clone())))
        },
    )
}

pub async fn get_watcher_statuses(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
) -> (
    StatusCode,
    Json<Option<Vec<(DateTime<Local>, watcher::Status)>>>,
) {
    app_state.read().await.watchers.get(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| {
            (
                StatusCode::OK,
                Json(Some(watcher.statuses.clone().collect())),
            )
        },
    )
}

pub async fn get_watcher_status(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path((name, index)): Path<(String, usize)>,
) -> (StatusCode, Json<Option<(DateTime<Local>, watcher::Status)>>) {
    app_state.read().await.watchers.get(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| {
            watcher.statuses.iter().rev().nth(index).map_or_else(
                || (StatusCode::NOT_FOUND, Json(None)),
                |status| (StatusCode::OK, Json(Some(status.clone()))),
            )
        },
    )
}

pub async fn post_watcher_status(
    State(app_state): State<Arc<RwLock<AppState>>>,
    Path(name): Path<String>,
    Json(status): Json<watcher::Status>,
) -> (StatusCode, Json<Option<watcher::Status>>) {
    let time = Local::now();
    app_state.write().await.watchers.get_mut(&name).map_or_else(
        || (StatusCode::NOT_FOUND, Json(None)),
        |watcher| {
            watcher.statuses.push((time, status.clone()));
            (StatusCode::CREATED, Json(Some(status)))
        },
    )
}
