mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use warp::Filter;
use watcher::{OKWhen, ServiceWatcher, Status};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    pond.add_watcher(
        "alright".to_string(),
        ServiceWatcher::new("https://google.com", OKWhen::Status(200)),
    )
    .unwrap();
    pond.add_watcher(
        "alwrong".to_string(),
        ServiceWatcher::new("https://matrixa.renn.es", OKWhen::Status(200)),
    )
    .unwrap();

    let statushistories: Arc<RwLock<HashMap<String, Vec<Status>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let max_history = 2;

    // Start background service watcher
    let watcher_handle = tokio::spawn(background_watcher(
        pond,
        statushistories.clone(),
        max_history,
    ));
    let service_handler = warp::path!("service" / String).map(move |name| {
        let histories = statushistories.read();
        match histories.get(&name) {
            Some(history) => warp::reply::json(&history),
            None => warp::reply::json(&Vec::<Status>::new()),
        }
    });
    warp::serve(service_handler)
        .run(([127, 0, 0, 1], 3030))
        .await;
    watcher_handle.abort();
}

async fn background_watcher(
    pond: ServiceWatcherPond,
    statushistories: Arc<RwLock<HashMap<String, Vec<Status>>>>,
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
                for status in result {
                    histories
                        .entry(status.name.clone())
                        .or_insert(Vec::new())
                        .push(status.status);
                    if histories[&status.name].len() > max_history {
                        histories.get_mut(&status.name).unwrap().remove(0); // Unwrap is safe because we just inserted it
                    }
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    }
}
