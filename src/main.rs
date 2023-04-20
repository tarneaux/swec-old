mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use std::time::Duration;
use tokio::time::sleep;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let watcher = ServiceWatcher::new(
        "http://github.com/tarneaux/",
        Duration::from_secs(5),
        OKWhen::InDom("supersplit".to_string()),
    );

    let mut pond = ServiceWatcherPond::new();
    pond.add_watcher(watcher);

    loop {
        pond.run().await;
        sleep(Duration::from_secs(5)).await;
    }
}
