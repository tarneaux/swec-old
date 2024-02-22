use reqwest::Client;
use swec::watcher::{Spec, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new reqwest client
    let client = Client::new();

    let api_url = "http://localhost:8081/api/v1";

    // Create a new watcher
    let spec = Spec::new("Example watcher".to_string(), None);
    let r = client
        .post(&format!("{api_url}/watchers/test/spec"))
        .json(&spec)
        .send()
        .await?;
    println!("Post watcher: {}", r.status());

    // Add a status to the watcher
    let status = Status {
        is_up: true,
        message: "Everything is fine".to_string(),
        time: chrono::Local::now(),
    };
    let r = client
        .post(&format!("{api_url}/watchers/test/statuses"))
        .json(&status)
        .send()
        .await?;
    println!("Post status: {}", r.status());

    // Get the watcher
    let r = client
        .get(&format!("{api_url}/watchers/test"))
        .send()
        .await?;
    println!("Get watcher: {}", r.status());
    let watcher: swec::watcher::Watcher = r.json().await?;
    println!("Watcher: {watcher:#?}");

    // Get the watcher's statuses
    let r = client
        .get(&format!("{api_url}/watchers/test/statuses"))
        .send()
        .await?;
    println!("Get statuses: {}", r.status());
    let statuses: Vec<Status> = r.json().await?;
    println!("Statuses: {statuses:#?}");

    Ok(())
}
