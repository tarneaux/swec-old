use reqwest::Client;
use swec::watcher::{Info, Status};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp = client
        .post("http://localhost:8081/watchers/test/spec")
        .json(&Info::new("test".to_string(), None))
        .send();
    let resp = resp.await?;
    std::println!("{}", resp.status());

    let resp = client
        .get("http://localhost:8081/watchers/test/spec")
        .send();
    let resp = resp.await?;
    let resp = resp.json::<Info>().await?;
    std::println!("{:?}", resp);

    let resp = client
        .post("http://localhost:8081/watchers/test/statuses")
        .json(&Status {
            is_up: true,
            message: "test".to_string(),
            time: chrono::Local::now(),
        })
        .send();

    let resp = resp.await?;
    std::println!("{}", resp.status());

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
    std::println!("{}", resp.status());

    let resp = client
        .get("http://localhost:8081/watchers/test/statuses")
        .send();
    let resp = resp.await?;
    let resp = resp.json::<Vec<Status>>().await?;
    std::println!("{:?}", resp);

    Ok(())
}
