use actix_web::{web, App, HttpServer};
use color_eyre::eyre::Result;
use serde_json;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};

mod api;
use swec::watcher;

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: config file and/or command line arguments
    let watchers_path = Path::new("watchers.json");
    let history_len = 10;

    eprintln!("Restoring watchers from file");

    let watchers = load_watchers(watchers_path).await.unwrap_or_else(|e| {
        eprintln!("Failed to restore watchers from file: {}", e);
        eprintln!("Starting with an empty set of watchers");
        BTreeMap::new()
    });

    let app_state = Arc::new(RwLock::new(api::AppState {
        watchers,
        history_len,
    }));

    let app_state_cloned = app_state.clone();
    let public_server = HttpServer::new(move || {
        let app_state_cloned = app_state_cloned.clone();
        App::new()
            .app_data(web::Data::new(app_state_cloned))
            .service(api::get_watcher_spec)
            .service(api::get_watcher_statuses)
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
            .service(api::get_watcher_spec)
            .service(api::post_watcher_spec)
            .service(api::put_watcher_spec)
            .service(api::get_watcher_statuses)
            .service(api::post_watcher_status)
    })
    .bind(("127.0.0.1", 8081))?
    .run();

    eprintln!("Starting servers");

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

    eprintln!("{}", end_message);

    eprintln!("Saving watchers to file");
    save_watchers(watchers_path, app_state.read().await.watchers.clone()).await?;

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

async fn save_watchers(path: &Path, watchers: BTreeMap<String, watcher::Watcher>) -> Result<()> {
    let mut file = tokio::fs::File::create(path).await?;
    let serialized = serde_json::to_string(&watchers)?;
    file.write_all(serialized.as_bytes()).await?;
    Ok(())
}

async fn load_watchers(path: &Path) -> Result<BTreeMap<String, watcher::Watcher>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    let deserialized = serde_json::from_slice(&contents)?;
    Ok(deserialized)
}
