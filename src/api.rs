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
    fn add_watcher(&mut self, name: String, watcher_spec: watcher::Spec) -> Result<()> {
        if self.watchers.contains_key(&name) {
            return Err(eyre!("Watcher already exists"));
        }
        self.watchers
            .insert(name, watcher::Watcher::new(watcher_spec, self.history_len));
        Ok(())
    }
}

#[get("/watchers")]
pub async fn get_watchers(app_state: web::Data<Arc<RwLock<AppState>>>) -> impl Responder {
    let watchers = &app_state.read().await.watchers;
    HttpResponse::Ok().json(watchers)
}

#[get("/watchers/{name}")]
pub async fn get_watcher(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.read().await.watchers.get(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| HttpResponse::Ok().json(watcher),
    )
}

#[get("/watchers/{name}/spec")]
pub async fn get_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.read().await.watchers.get(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| HttpResponse::Ok().json(&watcher.spec),
    )
}

#[post("/watchers/{name}/spec")]
pub async fn post_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
    spec: web::Json<watcher::Spec>,
) -> impl Responder {
    let name = path.into_inner();
    app_state
        .write()
        .await
        .add_watcher(name, spec.into_inner())
        .map_or_else(
            |_| HttpResponse::Conflict().finish(),
            |()| HttpResponse::Created().finish(),
        )
}

#[put("/watchers/{name}/spec")]
pub async fn put_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    path: web::Path<String>,
    spec: web::Json<watcher::Spec>,
) -> impl Responder {
    let name = path.into_inner();
    app_state.write().await.watchers.get_mut(&name).map_or_else(
        || HttpResponse::NotFound().body("Watcher not found"),
        |watcher| {
            watcher.spec = spec.into_inner();
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
