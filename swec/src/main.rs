use axum::Router;
use color_eyre::eyre::Result;
use std::collections::BTreeMap;
use std::future::IntoFuture;
use std::path::Path;
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};

mod api;
mod ringbuffer;
pub use ringbuffer::{RingBuffer, StatusRingBuffer};
use swec_core::{watcher, ApiInfo};
use tracing::{info, warn};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: config file and/or command line arguments
    let watchers_path = Path::new("watchers.json");
    let history_len = 3600;
    let truncate_histories = false;
    let public_address = "127.0.0.1:8080";
    let private_address = "127.0.0.1:8081";
    let api_path = "/api/v1";

    tracing_subscriber::fmt::init();

    info!("Restoring watchers from file");

    let watchers = load_watchers(watchers_path, history_len, truncate_histories)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to restore watchers from file: {e}");
            warn!("Starting with an empty set of watchers");
            BTreeMap::new()
        });

    let app_state = Arc::new(RwLock::new(api::AppState::new(watchers, history_len)));

    let public_server = {
        let router = Router::new()
            .nest(api_path, api::read_only_router())
            .with_state((
                ApiInfo {
                    writable: false,
                    swec_version: VERSION.to_string(),
                },
                app_state.clone(),
            ));
        let listener = tokio::net::TcpListener::bind(public_address).await?;
        axum::serve(listener, router.into_make_service()).into_future()
    };

    let private_server = {
        let router = Router::new()
            .nest(api_path, api::read_write_router())
            .with_state((
                ApiInfo {
                    writable: true,
                    swec_version: VERSION.to_string(),
                },
                app_state.clone(),
            ));
        let listener = tokio::net::TcpListener::bind(private_address).await?;
        axum::serve(listener, router.into_make_service()).into_future()
    };

    info!("Starting servers");

    let server_end_message = |v| match v {
        Ok(()) => "Server shut down without errors".to_string(),
        Err(e) => format!("Server shut down with error: {e}"),
    };

    // Wait for a server to shut down or for a stop signal to be received.
    let end_message = tokio::select! {
        v = public_server => server_end_message(v),
        v = private_server => server_end_message(v),
        () = wait_for_stop_signal() => "Interrupt received".to_string(),
    };

    info!("{end_message}");

    info!("Saving watchers to file");
    save_watchers(watchers_path, app_state.read().await.get_watchers().clone()).await?;

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
        .map(Box::pin)
        .collect::<Vec<_>>();

    futures::future::select_all(interrupt_futures).await;
}

async fn save_watchers(
    path: &Path,
    watchers: BTreeMap<String, watcher::Watcher<StatusRingBuffer>>,
) -> Result<()> {
    let mut file = tokio::fs::File::create(path).await?;
    let serialized = serde_json::to_string(&watchers)?;
    file.write_all(serialized.as_bytes()).await?;
    Ok(())
}

async fn load_watchers(
    path: &Path,
    history_length: usize,
    truncate: bool,
) -> Result<BTreeMap<String, watcher::Watcher<StatusRingBuffer>>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    let mut deserialized: BTreeMap<String, watcher::Watcher<StatusRingBuffer>> =
        serde_json::from_slice(&contents)?;
    // Make sure the histories are all the correct length
    for watcher in deserialized.values_mut() {
        if truncate {
            watcher.statuses.truncate_fifo(history_length);
        } else {
            watcher
                .statuses
                .resize(history_length)
                .expect("Failed to resize watcher history");
        }
    }
    Ok(deserialized)
}
