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
    ));

    pond.watchers.push(ServiceWatcher::new(
        "https://www.grstoogle.com",
        OKWhen::Status(200),
    ));

    let statushistories: Arc<RwLock<Vec<Vec<Status>>>> =
        Arc::new(RwLock::new(Vec::with_capacity(pond.watchers.len())));

    let max_history = 2;

    // Start background service watcher
    let watcher_handle = tokio::spawn(background_watcher(
        pond,
        statushistories.clone(),
        max_history,
    ));
    let service_handler = warp::path!("service" / usize).map(move |id| {
        let histories = statushistories.read();
        match histories.get(id) {
            Some(history) => warp::reply::json(&history),
            None => warp::reply::json(&Vec::<Status>::new()), // TODO: return error code to make a
                                                              // real REST API
        }
    });
    // TODO: also allow
    // getting service by
    // name
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
