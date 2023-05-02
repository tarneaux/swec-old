mod multi_watcher;
mod status_history;
mod watcher;
use status_history::StatusHistoryPond;
use std::time::Duration;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut pond = StatusHistoryPond::new(5);
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
        let statuses = pond.get_statuses(0);
        println!("{:?}", statuses);
    }
}
