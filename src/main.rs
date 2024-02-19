use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

mod watcher;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_state = Arc::new(RwLock::new(AppState::new(10)));

    let app_state_cloned = app_state.clone();
    let public_server = HttpServer::new(move || {
        let app_state_cloned = app_state_cloned.clone();
        App::new()
            .app_data(web::Data::new(app_state_cloned))
            .service(get_watcher_spec)
            .service(get_watcher_statuses)
    })
    .bind(("0.0.0.0", 8080))?
    .run();

    let app_state_cloned = app_state.clone();
    let private_server = HttpServer::new(move || {
        let app_state_cloned = app_state_cloned.clone();
        // TODO: just add private routes to the public server's App since the
        // private only has additional routes
        App::new()
            .app_data(web::Data::new(app_state_cloned))
            .service(create_watcher)
            .service(get_watcher_spec)
            .service(get_watcher_statuses)
            .service(create_watcher_status)
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
    watchers: BTreeMap<String, watcher::Watcher>,
    history_len: usize,
}

impl AppState {
    const fn new(history_len: usize) -> Self {
        Self {
            watchers: BTreeMap::new(),
            history_len,
        }
    }

    fn add_watcher(&mut self, name: String, watcher_spec: watcher::Info) -> Result<(), ()> {
        if self.watchers.contains_key(&name) {
            return Err(());
        } else {
            self.watchers
                .insert(name, watcher::Watcher::new(watcher_spec, self.history_len));
            Ok(())
        }
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
    info: web::Json<watcher::Info>,
) -> impl Responder {
    match app_state
        .write()
        .await
        .add_watcher(name.into_inner(), info.into_inner())
    {
        Ok(()) => HttpResponse::Created().finish(),
        Err(()) => HttpResponse::Conflict().finish(),
    }
}

#[get("/watchers/{name}/statuses")]
async fn get_watcher_statuses(
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
            |watcher| HttpResponse::Ok().json(&watcher.statuses),
        )
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum SingleOrVec<T> {
    Single(T),
    Multiple(Vec<T>),
}

impl<T> From<SingleOrVec<T>> for Vec<T> {
    fn from(om: SingleOrVec<T>) -> Self {
        match om {
            SingleOrVec::Single(t) => vec![t],
            SingleOrVec::Multiple(ts) => ts,
        }
    }
}

#[post("/watchers/{name}/statuses")]
async fn create_watcher_status(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    name: web::Path<String>,
    statuses: web::Json<SingleOrVec<watcher::Status>>,
) -> impl Responder {
    app_state
        .write()
        .await
        .watchers
        .get_mut(&name.into_inner())
        .map_or_else(
            || HttpResponse::NotFound().body("Watcher not found"),
            |watcher| {
                watcher.statuses.extend(Vec::from(statuses.into_inner()));
                HttpResponse::Created().finish()
            },
        )
}
