/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use clap::Parser;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio::sync::RwLock;
use warp::Filter;

mod argument_parser;
mod config;
mod status_handlers;
mod watchers;

use argument_parser::Args;
use config::Config;
use status_handlers::histfile::{
    read_histories_from_file, restore_histories_to_pond, HistfileStatusHandler,
};
use watchers::pond::ServiceWatcherPond;
use watchers::status::Status;
use watchers::ServiceWatcher;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = Config::read(&args.config).unwrap_or_else(|e| {
        eprintln!("Error while reading config file: {}", e);
        std::process::exit(1);
    });

    let histories = read_histories_from_file("./histfile");

    let file = match File::create("./histfile").await {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error while opening histfile: {}", e);
            std::process::exit(1);
        }
    };

    let pond = ServiceWatcherPond::new(
        config.watchers,
        config.histsize,
        Duration::from_secs(config.interval),
        vec![Box::new(HistfileStatusHandler::new(BufWriter::new(file)))],
    );

    let mut pond = match histories {
        Ok(histories) => restore_histories_to_pond(histories, pond).await,
        Err(e) => {
            eprintln!("Error while reading histfile: {}", e);
            pond
        }
    };

    let status_histories = pond.status_histories.clone();
    let watchers = pond.watchers.clone();

    let webserver_handle = tokio::spawn(async move {
        webserver(status_histories, watchers).await;
    });

    pond.watch().await;

    webserver_handle.await.unwrap_or_else(|e| {
        eprintln!("Error while running webserver: {:?}", e);
    });
}

async fn webserver(status_histories: Arc<RwLock<Vec<Vec<Status>>>>, watchers: Vec<ServiceWatcher>) {
    let service_handler = {
        // Get the status of a service
        let service_status_handler = {
            let status_histories = status_histories.clone();
            warp::path!("service" / usize / "status")
                .then(move |id| handle_service_status(status_histories.clone(), id))
        };

        // Get the status of all services
        let all_services_status_handler = {
            let status_histories = status_histories.clone();
            warp::path!("service" / "status")
                .then(move || handle_all_services_status(status_histories.clone()))
        };

        // Get the information of a service
        let service_info_handler = {
            let watchers = watchers.clone();
            warp::path!("service" / usize / "info")
                .then(move |id| handle_service_info(watchers.clone(), id))
        };

        // Get the information of all services
        let all_services_info_handler = {
            let watchers = watchers.clone();
            warp::path!("service" / "info").map(move || warp::reply::json(&watchers))
        };

        service_status_handler
            .or(all_services_status_handler)
            .or(service_info_handler)
            .or(all_services_info_handler)
    };

    warp::serve(service_handler)
        .run(([127, 0, 0, 1], 3030))
        .await;
}

async fn handle_service_status(
    status_histories: Arc<RwLock<Vec<Vec<Status>>>>,
    id: usize,
) -> impl warp::Reply {
    let history = status_histories.read().await.get(id).cloned();
    history.map_or_else(
        || {
            warp::reply::with_status(
                warp::reply::json(&"Not found"),
                warp::http::StatusCode::NOT_FOUND,
            )
        },
        |history| warp::reply::with_status(warp::reply::json(&history), warp::http::StatusCode::OK),
    )
}

async fn handle_all_services_status(
    status_histories: Arc<RwLock<Vec<Vec<Status>>>>,
) -> impl warp::Reply {
    let histories = status_histories.read().await.clone();
    warp::reply::with_status(warp::reply::json(&histories), warp::http::StatusCode::OK)
}

async fn handle_service_info(watchers: Vec<ServiceWatcher>, id: usize) -> impl warp::Reply {
    watchers.get(id).map_or_else(
        || {
            warp::reply::with_status(
                warp::reply::json(&"Not found"),
                warp::http::StatusCode::NOT_FOUND,
            )
        },
        |watcher| warp::reply::with_status(warp::reply::json(&watcher), warp::http::StatusCode::OK),
    )
}
