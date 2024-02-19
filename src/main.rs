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
    const fn new(history_len: usize) -> Self {
        Self {
            watchers: BTreeMap::new(),
            history_len,
        }
    }

    fn add_watcher(&mut self, name: String, watcher_spec: WatcherInfo) {
        self.watchers
            .insert(name, Watcher::new(watcher_spec, self.history_len));
    }
}

#[get("/watchers/{name}/spec")]
async fn get_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    name: web::Path<String>,
) -> impl Responder {
    app_state
        .read()
        .await
        .watchers
        .get(&name.into_inner())
        .map_or_else(
            || HttpResponse::NotFound().body("Watcher not found"),
            |watcher| HttpResponse::Ok().json(&watcher.info),
        )
}

#[post("/watchers/{name}/spec")]
async fn create_watcher(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    name: web::Path<String>,
    info: web::Json<WatcherInfo>,
) -> impl Responder {
    app_state
        .write()
        .await
        .add_watcher(name.into_inner(), info.into_inner());
    HttpResponse::Created()
}

struct Watcher {
    info: WatcherInfo,
    /// History of the status of the service
    history: WatcherHistory,
}

impl Watcher {
    /// Create a new watcher with an empty history.
    fn new(info: WatcherInfo, hist_len: usize) -> Self {
        Self {
            info,
            history: WatcherHistory::new(hist_len),
        }
    }
}

/// Information about a service. Only intended to be read by humans.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct WatcherInfo {
    /// Description of the service
    description: String,
    /// URL of the service, if applicable
    url: Option<String>,
    // TODO: service groups with a Group struct
}

impl WatcherInfo {
    const fn new(description: String, url: Option<String>) -> Self {
        Self { description, url }
    }
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
