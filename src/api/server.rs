use crate::watchers::{TimeStampedStatus, Watcher};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

pub async fn api_server(
    status_histories: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
    watchers: Vec<Watcher>,
) {
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
    status_histories: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
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
    status_histories: Arc<RwLock<Vec<Vec<TimeStampedStatus>>>>,
) -> impl warp::Reply {
    let histories = status_histories.read().await.clone();
    warp::reply::with_status(warp::reply::json(&histories), warp::http::StatusCode::OK)
}

async fn handle_service_info(watchers: Vec<Watcher>, id: usize) -> impl warp::Reply {
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
