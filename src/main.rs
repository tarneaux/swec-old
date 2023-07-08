use clap::Parser;
use warp::Filter;

mod argument_parser;
mod multi_watcher;
mod watcher;

use argument_parser::Args;
use multi_watcher::ServiceWatcherPond;

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut pond = ServiceWatcherPond::new_from_config(&args.config).unwrap_or_else(|err| {
        eprintln!("{}", err);
        std::process::exit(1);
    });

    let statushistories = pond.statushistories.clone();

    let watcher_handle = pond.start_watcher();

    let service_handler = {
        // Get the status of a service
        let service_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / usize / "status").map(move |id| {
                let history = statushistories.read().get(id).cloned();
                history.map_or_else(
                    || {
                        warp::reply::with_status(
                            warp::reply::json(&"Not found"),
                            warp::http::StatusCode::NOT_FOUND,
                        )
                    },
                    |history| {
                        warp::reply::with_status(
                            warp::reply::json(&history),
                            warp::http::StatusCode::OK,
                        )
                    },
                )
            })
        };

        // Get the status of all services
        let all_services_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / "statuses").map(move || {
                let histories = statushistories.read().clone();
                warp::reply::json(&histories)
            })
        };

        // Get the information of a service
        let service_info_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / usize / "info").map(move |id| {
                watchers.get(id).map_or_else(
                    || {
                        warp::reply::with_status(
                            warp::reply::json(&"Not found"),
                            warp::http::StatusCode::NOT_FOUND,
                        )
                    },
                    |watcher| {
                        warp::reply::with_status(
                            warp::reply::json(&watcher),
                            warp::http::StatusCode::OK,
                        )
                    },
                )
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
