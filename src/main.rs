use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(RwLock::new(AppState::new(10)));

    let app_state_cloned = app_state.clone();
    let public_server = HttpServer::new(move || {
        let app_state_cloned = app_state_cloned.clone();
        App::new().app_data(web::Data::new(app_state_cloned))
    })
    .bind(("0.0.0.0", 8080))?
    .run();

    let app_state_cloned = app_state.clone();
    let private_server = HttpServer::new(move || {
        let app_state_cloned = app_state_cloned.clone();
        App::new()
            .app_data(web::Data::new(app_state_cloned))
            .service(create_watcher)
    })
    .bind(("127.0.0.1", 8081))?
    .run();
    tokio::select! {
        _ = public_server => {},
        _ = private_server => {},
    }
    Ok(())
}

struct AppState {
    watchers: BTreeMap<String, Watcher>,
    history_len: usize,
}

impl AppState {
    fn new(history_len: usize) -> Self {
        Self {
            watchers: BTreeMap::new(),
            history_len,
        }
    }
}

#[get("/watcher_spec/{name}")]
async fn get_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    name: web::Path<String>,
) -> impl Responder {
    let name = name.into_inner();
    let app_state = app_state.read().await;
    let watcher = app_state.watchers.get(&name);
    match watcher {
        Some(watcher) => HttpResponse::Ok().json(watcher.spec.clone()),
        None => HttpResponse::NotFound().body("Watcher not found"),
    }
}

#[post("/watcher_spec")]
async fn create_watcher(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    watcher_spec: web::Json<WatcherSpec>,
) -> impl Responder {
    app_state.write().await.watchers.insert(
        watcher_spec.name.clone(),
        Watcher::new(watcher_spec.into_inner(), 10),
    );
    HttpResponse::Created()
}

struct Watcher {
    spec: WatcherSpec,
    history: WatcherHistory,
}

impl Watcher {
    /// Create a new watcher with an empty history.
    fn new(spec: WatcherSpec, hist_len: usize) -> Self {
        Self {
            spec,
            history: WatcherHistory::new(hist_len),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WatcherSpec {
    /// The name of the watcher
    name: String,
    /// Human readable information about the watcher
    information: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct WatcherHistory(VecDeque<Status>);

impl WatcherHistory {
    /// Create a new empty history with a given length.
    fn new(hist_len: usize) -> Self {
        Self(VecDeque::with_capacity(hist_len))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Status {
    /// Whether the service is up or down
    is_up: bool,
    /// Human readable information about the status
    message: String,
    /// The time the status was recorded
    time: DateTime<Local>,
}
