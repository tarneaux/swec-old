mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use warp::Filter;
use watcher::{OKWhen, ServiceWatcher, Status};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    pond.watchers.push(ServiceWatcher::new(
        "https://www.google.com",
        OKWhen::Status(200),
        "google",
    ));

    pond.watchers.push(ServiceWatcher::new(
        "https://www.grstoogle.com",
        OKWhen::Status(200),
        "grstoogle",
    ));

    let statushistories: Arc<RwLock<Vec<Vec<Status>>>> =
        Arc::new(RwLock::new(Vec::with_capacity(pond.watchers.len())));

    let max_history = 86400;

    // Start background service watcher
    let watcher_handle = tokio::spawn(background_watcher(
        pond.clone(),
        statushistories.clone(),
        max_history,
        Duration::from_secs(1),
    ));

    let service_handler = {
        // Get the status of a service
        let service_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / usize / "status").map(move |id| {
                let histories = statushistories.read();
                match histories.get(id) {
                    Some(history) => warp::reply::json(&history),
                    None => warp::reply::json(&Vec::<Status>::new()),
                }
            })
        };

        // Get the status of all services
        let all_services_status_handler = {
            let statushistories = statushistories.clone();
            warp::path!("service" / "statuses").map(move || {
                let histories = statushistories.read();
                let histories: Vec<_> = histories.iter().map(|h| h.clone()).collect();
                warp::reply::json(&histories)
            })
        };

        // Get the name of a service
        let service_name_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / usize / "name").map(move |id| match watchers.get(id) {
                Some::<&ServiceWatcher>(watcher) => {
                    let name: String = watcher.name.clone();
                    name
                }
                None => "".to_string(),
            })
        };

        // Get the names of all services
        let all_services_name_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / "names").map(move || {
                let mut result = Vec::new();
                for watcher in watchers.iter() {
                    result.push(&watcher.name);
                }
                warp::reply::json(&result)
            })
        };

        service_status_handler
            .or(all_services_status_handler)
            .or(service_name_handler)
            .or(all_services_name_handler)
    };

    warp::serve(service_handler)
        .run(([127, 0, 0, 1], 3030))
        .await;
    watcher_handle.abort();
}

async fn background_watcher(
    pond: ServiceWatcherPond,
    statushistories: Arc<RwLock<Vec<Vec<Status>>>>,
    max_history: usize,
    interval: Duration,
) {
    loop {
        let start_time = tokio::time::Instant::now();
        let result = pond.run(interval).await;

        if let Err(e) = result {
            println!("Error: {:?}", e);
            continue;
        } else {
            let result = result.unwrap();
            {
                let mut histories = statushistories.write();
                for (id, status) in result.iter().enumerate() {
                    if histories.len() <= id {
                        histories.push(Vec::new());
                    }
                    let history = &mut histories[id];
                    history.push(status.clone());
                    if history.len() > max_history {
                        history.remove(0);
                    }
                }
            }
        }
        let elapsed = start_time.elapsed();
        if elapsed < interval {
            sleep(interval - elapsed).await;
        }
    }
}
