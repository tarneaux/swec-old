mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use std::time::Duration;
use tokio::time::sleep;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    pond.add_watcher(ServiceWatcher::new(
        "http://github.com/tarneaux/",
        Duration::from_secs(5),
        OKWhen::InDom("supersplit".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://google.com/",
        Duration::from_secs(5),
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        Duration::from_secs(5),
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        Duration::from_secs(5),
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        Duration::from_secs(5),
        OKWhen::InDom("google".to_string()),
    ));

    loop {
        pond.run().await;
        pond.get_last_statuses().await;
        sleep(Duration::from_secs(5)).await;
    }
}
