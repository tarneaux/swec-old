use actix_web::{get, post, put, web, HttpResponse, Responder};
use color_eyre::eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use swec::watcher;

pub struct AppState {
    pub watchers: BTreeMap<String, watcher::Watcher>,
    pub history_len: usize,
}

impl AppState {
    fn add_watcher(&mut self, name: String, watcher_spec: watcher::Info) -> Result<()> {
        if self.watchers.contains_key(&name) {
            return Err(eyre!("Watcher already exists"));
        } else {
            self.watchers
                .insert(name, watcher::Watcher::new(watcher_spec, self.history_len));
            Ok(())
        }
    }
}

#[get("/watchers/{name}/spec")]
pub async fn get_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.read().await.watchers.get(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| HttpResponse::Ok().json(&watcher.info),
    )
}

#[post("/watchers/{name}/spec")]
pub async fn post_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
    info: web::Json<watcher::Info>,
) -> impl Responder {
    let name = path.into_inner();
    match app_state.write().await.add_watcher(name, info.into_inner()) {
        Ok(()) => HttpResponse::Created().finish(),
        Err(_) => HttpResponse::Conflict().finish(),
    }
}

#[put("/watchers/{name}/spec")]
pub async fn put_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
    info: web::Json<watcher::Info>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.write().await.watchers.get_mut(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| {
            watcher.info = info.into_inner();
            HttpResponse::NoContent().finish()
        },
    )
}

#[get("/watchers/{name}/statuses")]
pub async fn get_watcher_statuses(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.read().await.watchers.get(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| HttpResponse::Ok().json(&watcher.statuses),
    )
}

#[get("/watchers/{name}/statuses/{index}")]
pub async fn get_watcher_status(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<(String, usize)>,
) -> impl Responder {
    let (name, index) = path.into_inner();
    app_state.read().await.watchers.get(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| {
            watcher.statuses.iter().rev().nth(index).map_or_else(
                || HttpResponse::NotFound().body("Status not found"),
                |status| HttpResponse::Ok().json(status),
            )
        },
    )
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SingleOrVec<T> {
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
pub async fn post_watcher_status(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
    statuses: web::Json<SingleOrVec<watcher::Status>>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.write().await.watchers.get_mut(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| {
            watcher
                .statuses
                .push_multiple(Vec::from(statuses.into_inner()));
            HttpResponse::Created().finish()
        },
    )
}
