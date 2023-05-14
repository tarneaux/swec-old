mod multi_watcher;
mod watcher;
use multi_watcher::ServiceWatcherPond;
use std::time::Duration;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut pond = ServiceWatcherPond::new();
    pond.add_watcher(
        "alright".to_string(),
        ServiceWatcher::new("google.com", OKWhen::Status(200)),
    )
    .unwrap();
    pond.add_watcher(
        "alwrong".to_string(),
        ServiceWatcher::new("googlearstarst.com", OKWhen::Status(200)),
    )
    .unwrap();
    let timeout = Duration::from_secs(5);
    let statuses = pond.run(timeout).await;
    println!("{:?}", statuses);
}
