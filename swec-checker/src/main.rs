use clap::Parser;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use swec_client::{ReadApi, ReadWriteClient, WriteApi};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let client = ReadWriteClient::new(args.api_url);
    // Make sure the watcher exists
    if client.get_watcher(&args.name).await.is_err() {
        client
            .post_watcher_spec(
                &args.name,
                swec_core::Spec {
                    description: args.description.clone(),
                    url: None, // TODO
                },
            )
            .await
            .unwrap();
    }
    loop {
        let status = args.checker.check(args.timeout).await;
        client
            .post_watcher_status(&args.name, status)
            .await
            .unwrap();
        tokio::time::sleep(tokio::time::Duration::from_secs(args.interval)).await;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Checker {
    Http { url: String },
}

impl Checker {
    async fn check(&self, timeout: u64) -> swec_core::Status {
        match self {
            Self::Http { url } => {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(timeout))
                    .build()
                    .unwrap();
                match client.get(url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            swec_core::Status {
                                is_up: true,
                                message: "Success".to_string(),
                            }
                        } else {
                            swec_core::Status {
                                is_up: false,
                                message: format!("HTTP error: {}", response.status()),
                            }
                        }
                    }
                    Err(e) => swec_core::Status {
                        is_up: false,
                        message: format!("Error: {}", e),
                    },
                }
            }
        }
    }
}

/// Create a `Checker` from a string.
/// The string should be in the format `http:<url>`.
impl FromStr for Checker {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        match parts.as_slice() {
            ["http", url] => Ok(Self::Http {
                url: (*url).to_string(),
            }),
            _ => Err(format!("Invalid checker: {s}")),
        }
    }
}

#[derive(Clone, Parser, Debug)]
#[command(version, about, author, long_about)]
struct Args {
    name: String,
    description: String,
    checker: Checker,
    #[clap(short, long, default_value = "5")]
    interval: u64,
    #[clap(short, long, default_value = "10")]
    timeout: u64,
    #[clap(short, long, default_value = "http://localhost:8081/api/v1")]
    api_url: String,
}
