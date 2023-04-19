mod monitor;
use monitor::Watcher;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let url = "https://www.google.com";
    let mut service = Watcher::new(
        url.to_string(),
        200,
        10,
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    service.check_health().await;
    let last_check = service.get_last_check();
    match last_check {
        Some(check) => match check.get_ping() {
            Some(ping) => println!("Ping: {}", ping),
            None => println!("No ping"),
        },
        None => println!("No checks"),
    }
}
