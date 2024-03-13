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
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod api;
mod ringbuffer;
pub use ringbuffer::{RingBuffer, StatusRingBuffer};
use swec_core::{checker, ApiInfo};
use tracing::{info, warn};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: config file and/or command line arguments
    let checkers_path = Path::new("checkers.json");
    let history_len = 3600;
    let truncate_histories = false;
    let public_address = "127.0.0.1:8080";
    let private_address = "127.0.0.1:8081";
    let api_path = "/api/v1";

    tracing_subscriber::fmt::init();

    info!("Restoring checkers from file");

    let checkers = load_checkers(checkers_path, history_len, truncate_histories)
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to restore checkers from file: {e}");
            warn!("Starting with an empty set of checkers");
            BTreeMap::new()
        });

    let app_state = Arc::new(RwLock::new(api::AppState::new(checkers, history_len)));

    let public_server = {
        let router = Router::new()
            .nest(api_path, api::read_only_router())
            .with_state((
                ApiInfo {
                    writable: false,
                    swec_version: VERSION.to_string(),
                },
                app_state.clone(),
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            );
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
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::default().include_headers(true)),
            );
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

    info!("Saving checkers to file");

    let res = app_state.read().await.checkers_to_json();
    match res {
        Ok(json) => save_checkers(checkers_path, json).await?,
        Err(e) => warn!("Failed to save checkers to file: {e}"),
    }

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

async fn save_checkers(path: &Path, serialized: String) -> Result<()> {
    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(serialized.as_bytes()).await?;
    Ok(())
}

async fn load_checkers(
    path: &Path,
    history_length: usize,
    truncate: bool,
) -> Result<BTreeMap<String, checker::Checker<StatusRingBuffer>>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;
    let mut deserialized: BTreeMap<String, checker::Checker<StatusRingBuffer>> =
        serde_json::from_slice(&contents)?;
    // Make sure the histories are all the correct length
    for checker in deserialized.values_mut() {
        if truncate {
            checker.statuses.truncate_fifo(history_length);
        } else {
            checker
                .statuses
                .resize(history_length)
                .expect("Failed to resize checker history");
        }
    }
    Ok(deserialized)
}
