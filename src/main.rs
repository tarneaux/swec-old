mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use std::time::Duration;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    pond.add_watcher(ServiceWatcher::new(
        "http://github.com/tarneaux/",
        OKWhen::InDom("supersplit".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://google.com/",
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        OKWhen::InDom("google".to_string()),
    ));

    pond.add_watcher(ServiceWatcher::new(
        "http://arstarsttthngoogle.com/",
        OKWhen::InDom("google".to_string()),
    ));

    let timeout = Duration::from_secs(5);

    loop {
        pond.run(timeout).await;
        let statuses = pond.get_last_statuses().await;
        println!("{:?}", statuses);
    }
}
