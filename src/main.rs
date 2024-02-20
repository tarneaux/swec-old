use actix_web::{get, post, put, web, App, HttpResponse, HttpServer, Responder};
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
            .service(get_watcher_spec)
            .service(post_watcher_spec)
            .service(put_watcher_spec)
            .service(get_watcher_statuses)
            .service(post_watcher_status)
    })
    .bind(("127.0.0.1", 8081))?
    .run();

    // Wait for a server to shut down or for a stop signal to be received.
    let end_message = tokio::select! {
        _ = public_server => {
            "Public server shut down, shutting down private server"
        },
        _ = private_server => {
            "Private server shut down, shutting down public server"
        },
        _ = wait_for_stop_signal() => {
            "Interrupt received, shutting down servers"
        },
    };

    println!("{}", end_message);

    Ok(())
}

/// Wait for a stop signal to be received.
async fn wait_for_stop_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let interrupt_signal_kinds = vec![
        SignalKind::alarm(),
        SignalKind::hangup(),
        SignalKind::interrupt(),
        SignalKind::pipe(),
        SignalKind::quit(),
        SignalKind::terminate(),
    ];
    let interrupt_futures = interrupt_signal_kinds
        .into_iter()
        .map(|kind| async move {
            // Because recv borrows the signal, we need to make a new future:
            // this allows keeping ownership of the signal until the future is
            // dropped, instead of dropping early in the map, when recv is
            // called (which would not work).
            signal(kind).expect("Failed to create signal").recv().await;
        })
        .map(|future| Box::pin(future))
        .collect::<Vec<_>>();

    futures::future::select_all(interrupt_futures).await;
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
async fn post_watcher_spec(
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

#[put("/watchers/{name}/spec")]
async fn put_watcher_spec(
    app_state: web::Data<Arc<RwLock<AppState>>>,
    name: web::Path<String>,
    info: web::Json<watcher::Info>,
) -> impl Responder {
    app_state
        .write()
        .await
        .watchers
        .get_mut(&name.into_inner())
        .map_or_else(
            || HttpResponse::NotFound().body("Watcher not found"),
            |watcher| {
                watcher.info = info.into_inner();
                HttpResponse::NoContent().finish()
            },
        )
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
async fn post_watcher_status(
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
