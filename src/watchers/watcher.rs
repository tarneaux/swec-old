use super::ok_when::OkWhen;
use super::status::{DownReason, Status};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Watcher {
    pub url: String,
    #[serde(default)]
    pub ok_when: OkWhen,
    pub name: String,
}

impl Watcher {
    pub async fn get_current_status(&self, timeout: &Duration) -> Status {
        let res = self.get_url(timeout).await;
        match res {
            Ok((res, duration)) => self
                .verify_status_or_content(res)
                .await
                .map_or_else(|| Status::Up(duration), Status::Down),
            Err(e) => Status::Down(e),
        }
    }

    async fn get_url(
        &self,
        timeout: &Duration,
    ) -> Result<(reqwest::Response, Duration), DownReason> {
        let client = Client::new();
        let start_time = std::time::Instant::now();
        let res = client.get(&self.url).timeout(*timeout).send().await;
        let end_time = std::time::Instant::now();
        let duration = end_time - start_time;
        res.map_or_else(
            |e| {
                if e.is_timeout() {
                    Err(DownReason::Timeout)
                } else {
                    eprintln!(
                        "Status of {} is Down::Unknown. Error returned by reqwest: {}",
                        self.name, e
                    );
                    Err(DownReason::Unknown)
                }
            },
            |res| Ok((res, duration)),
        )
    }

    async fn verify_status_or_content(&self, res: reqwest::Response) -> Option<DownReason> {
        if let Some(status) = self.ok_when.status {
            if res.status().as_u16() != status {
                return Some(DownReason::WrongStatus);
            }
        }
        let body = res.text().await.unwrap_or_else(|e| {
            eprintln!("Error while reading response body: {}", e);
            String::new() // Check will fail because we search in an empty string
        });
        if let Some(content) = &self.ok_when.content {
            if !body.contains(content) {
                return Some(DownReason::WrongContent);
            }
        }
        if !self.ok_when.content_regex.is_match(&body) {
            return Some(DownReason::WrongContent);
        }
        None
    }
}
