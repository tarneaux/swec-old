use warp::Filter;

mod multi_watcher;
mod watcher;

use multi_watcher::ServiceWatcherPond;
use watcher::Status;

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new_from_config("config.yaml").unwrap();

    let statushistories = pond.statushistories.clone();

    let watcher_handle = pond.start_watcher();

    let service_handler = {
        // Get the status of a service
        let service_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / usize / "status").map(move |id| {
                let histories = statushistories.read();
                match histories.get(id) {
                    Some(history) => warp::reply::with_status(
                        warp::reply::json(&history),
                        warp::http::StatusCode::OK,
                    ),
                    None => warp::reply::with_status(
                        warp::reply::json(&Vec::<Status>::new()),
                        warp::http::StatusCode::NOT_FOUND,
                    ),
                }
            })
        };

        // Get the status of all services
        let all_services_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / "statuses").map(move || {
                let histories = statushistories.read();
                // RwLockReadGuard<Vec<_>> -> Vec<_> to be able to serialize
                let histories: Vec<_> = histories.iter().cloned().collect();
                warp::reply::json(&histories)
            })
        };

        // Get the information of a service
        let service_info_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / usize / "info").map(move |id| match watchers.get(id) {
                Some(watcher) => warp::reply::with_status(
                    warp::reply::json(&watcher),
                    warp::http::StatusCode::OK,
                ),
                None => warp::reply::with_status(
                    warp::reply::json(&Vec::<Status>::new()),
                    warp::http::StatusCode::NOT_FOUND,
                ),
            })
        };

        // Get the information of all services
        let all_services_info_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / "infos").map(move || warp::reply::json(&watchers))
        };

        service_status_handler
            .or(all_services_status_handler)
            .or(service_info_handler)
            .or(all_services_info_handler)
    };

    warp::serve(service_handler)
        .run(([127, 0, 0, 1], 3030))
        .await;
    watcher_handle.abort();
}
