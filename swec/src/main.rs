use axum::Router;
use color_eyre::eyre::Result;
use std::collections::BTreeMap;
use std::future::IntoFuture;
use std::path::Path;
use std::sync::Arc;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufWriter, SeekFrom},
    signal::unix::{signal, SignalKind},
    sync::RwLock,
    time::Duration,
};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

mod api;
mod ringbuffer;
pub use ringbuffer::{RingBuffer, StatusRingBuffer};
use swec_core::{checker, ApiInfo};
use tracing::{error, info, warn};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<()> {
    // TODO: config file and/or command line arguments
    let checkers_path = Path::new("swec_dump.json");
    let history_len = 3600;
    let truncate_histories = false;
    let public_address = "127.0.0.1:8080";
    let private_address = "127.0.0.1:8081";
    let api_path = "/api/v1";
    let dump_interval = Duration::from_secs(60);

    tracing_subscriber::fmt::init();

    info!("Restoring checkers from dump file");

    let checkers = restore_checkers(checkers_path, history_len, truncate_histories)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to restore checkers from dump file: {e}, exiting.");
            error!("The only case where we will allow restoring to fail is if the file is empty, in which case we will just start with no checkers.");
            std::process::exit(1);
        });

    let state_writer = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(checkers_path)
        .await?;

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

    let dumper = {
        let app_state = app_state.clone();
        let writer = BufWriter::new(state_writer.try_clone().await?);
        tokio::spawn(dumper_task(app_state, writer, dump_interval))
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
        _ = dumper => unreachable!(),
        () = wait_for_stop_signal() => "Interrupt received".to_string(),
    };

    info!("{end_message}");

    // Save the checkers to file before exiting
    dump_checkers(&app_state, &mut BufWriter::new(state_writer))
        .await
        .unwrap_or_else(|e| {
            warn!("Failed to dump checkers to file: {e}");
        });

    Ok(())
}

/// Wait for a stop signal to be received.
async fn wait_for_stop_signal() {
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

async fn dump_checkers(
    app_state: &Arc<RwLock<api::AppState>>,
    writer: &mut BufWriter<File>,
) -> Result<()> {
    info!("Saving checkers to file");
    let serialized = app_state.read().await.checkers_to_json()?;
    (*writer).seek(SeekFrom::Start(0)).await?; // super important, otherwise we just append to the file
    (*writer).write_all(serialized.as_bytes()).await?;
    (*writer).flush().await?;
    Ok(())
}

async fn dumper_task(
    app_state: Arc<RwLock<api::AppState>>,
    mut writer: BufWriter<File>,
    interval: Duration,
) -> ! {
    let make_signal =
        || signal(SignalKind::user_defined1()).expect("Failed to create signal for dumper task");
    let mut s = make_signal();
    loop {
        tokio::select! {
            v = s.recv() => {
                if v.is_none() {
                    warn!("Cannot receive signals from this channel anymore, creating a new one");
                    s = make_signal();
                }
                info!("Received SIGUSR1, dumping checkers to file");
            }
            () = tokio::time::sleep(interval) => {}
        }
        dump_checkers(&app_state, &mut writer)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to dump checkers to file: {e}");
            });
    }
}

async fn restore_checkers(
    path: &Path,
    history_length: usize,
    truncate: bool,
) -> Result<BTreeMap<String, checker::Checker<StatusRingBuffer>>> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;

    if contents.is_empty() {
        // We can safely say that the user has just cleared the file or just installed swec,
        // which means we can return an empty map.
        return Ok(BTreeMap::new());
    }

    let mut deserialized: BTreeMap<String, checker::Checker<StatusRingBuffer>> =
        serde_json::from_slice(&contents)?;

    // Make sure the histories all have the correct length, since deserializing a ring buffer
    // doesn't guarantee that the history will be the correct length, plus the user might have
    // changed the history length between dumping and restoring.
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
