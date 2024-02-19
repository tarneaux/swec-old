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

    Ok(())
}
