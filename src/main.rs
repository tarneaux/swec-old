mod monitor;
use monitor::{OKWhen, ServiceWatcher};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    let mut watcher = ServiceWatcher::new(
        "http://google.com",
        Duration::from_secs(5),
        OKWhen::Status(200),
    );

    loop {
        let status = watcher.get_current_status().await;
        println!("Status: {:?}", status);
        sleep(Duration::from_secs(1)).await;
    }
}
