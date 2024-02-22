use reqwest::Client;
use swec::watcher::{Info, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new reqwest client
    let client = Client::new();

    // Create a new watcher with a description and no URL
    let resp = client
        .post("http://localhost:8081/watchers/test/spec")
        .json(&Info::new("test".to_string(), None))
        .send();
    let resp = resp.await?;
    std::println!("Post spec: {}", resp.status());

    // Update the watcher's spec
    let resp = client
        .put("http://localhost:8081/watchers/test/spec")
        .json(&Info::new(
            "test".to_string(),
            Some("http://localhost:8081".to_string()),
        ))
        .send();
    let resp = resp.await?;
    std::println!("Put spec: {}", resp.status());

    // Get the watcher's spec
    let resp = client
        .get("http://localhost:8081/watchers/test/spec")
        .send();
    let resp = resp.await?;
    std::println!("Get spec: {:?}", resp.json::<Info>().await?);

    // Add a status to the watcher
    let resp = client
        .post("http://localhost:8081/watchers/test/statuses")
        .json(&Status {
            is_up: true,
            message: "test".to_string(),
            time: chrono::Local::now(),
        })
        .send();

    let resp = resp.await?;
    std::println!("Post status: {}", resp.status());

    // Add multiple statuses to the watcher
    let resp = client
        .post("http://localhost:8081/watchers/test/statuses")
        .json(&vec![
            Status {
                is_up: true,
                message: "test".to_string(),
                time: chrono::Local::now(),
            },
            Status {
                is_up: false,
                message: "test".to_string(),
                time: chrono::Local::now(),
            },
        ])
        .send();
    let resp = resp.await?;
    std::println!("Post two statuses: {}", resp.status());

    // Get the watcher's statuses
    let resp = client
        .get("http://localhost:8081/watchers/test/statuses")
        .send();
    let resp = resp.await?;
    std::println!("Get statuses: {:?}", resp.json::<Vec<Status>>().await?);

    // Get a specific status
    let resp = client
        .get("http://localhost:8081/watchers/test/statuses/0")
        .send();
    let resp = resp.await?;
    std::println!("Get status: {:?}", resp.json::<Status>().await?);

    Ok(())
}
