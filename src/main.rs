mod watcher;
use std::time::Duration;
use tokio::time::sleep;
use watcher::{OKWhen, ServiceWatcher};

#[tokio::main]
async fn main() {
    let mut watcher = ServiceWatcher::new(
        "http://github.com/tarneaux/",
        Duration::from_secs(5),
        OKWhen::InDom("supersplit".to_string()),
    );

    loop {
        let status = watcher.get_current_status().await;
        println!("Status: {:?}", status);
        sleep(Duration::from_secs(1)).await;
    }
}
