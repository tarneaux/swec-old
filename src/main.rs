mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use warp::Filter;
use watcher::{ErrorType, OKWhen, ServiceWatcher, Status};

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

    let max_history = 2;

    // Start background service watcher
    let watcher_handle = tokio::spawn(background_watcher(
        pond.clone(),
        statushistories.clone(),
        max_history,
    ));

    let service_handler = {
        let statushistories = statushistories.clone();
        let service_status_handler = warp::path!("service" / usize / "status").map(move |id| {
            let histories = statushistories.read();
            match histories.get(id) {
                Some(history) => warp::reply::json(&history),
                None => warp::reply::json(&Vec::<Status>::new()), // TODO: return error code if
                                                                  // id is out of bounds
            }
        });
        let watchers = pond.watchers.clone();
        let service_info_handler = warp::path!("service" / usize / "name").map(move |id| {
            match watchers.get(id) {
                Some::<&ServiceWatcher>(watcher) => {
                    let name: String = watcher.name.clone();
                    name
                }
                None => "".to_string(), // TODO: return error code if
                                        // id is out of bounds
            }
        });
        // Get names of all services
        let service_list_handler = {
            let watchers = pond.watchers.clone();
            warp::path!("service" / "list").map(move || {
                let mut result = Vec::new();
                for watcher in watchers.iter() {
                    result.push(&watcher.name);
                }
                warp::reply::json(&result)
            })
        };
        service_status_handler
            .or(service_info_handler)
            .or(service_list_handler)
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
) {
    let timeout = Duration::from_secs(5);
    loop {
        let result = pond.run(timeout).await;

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
            sleep(Duration::from_secs(1)).await;
        }
    }
}
