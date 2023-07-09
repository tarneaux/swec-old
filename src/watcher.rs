/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceWatcher {
    pub url: String,
    #[serde(default)]
    pub ok_when: OkWhen,
    pub name: String,
}

impl ServiceWatcher {
    pub async fn get_current_status(&self, timeout: &Duration) -> Status {
        let res = self.get_url(timeout).await;
        match res {
            Ok((res, duration)) => self
                .verify_status_or_content(res)
                .await
                .map_or_else(|| Status::Online(duration), Status::Offline),
            Err(e) => Status::Offline(e),
        }
    }

    async fn get_url(
        &self,
        timeout: &Duration,
    ) -> Result<(reqwest::Response, Duration), ErrorType> {
        let client = Client::new();
        let start_time = std::time::Instant::now();
        let res = client.get(&self.url).timeout(*timeout).send().await;
        let end_time = std::time::Instant::now();
        let duration = end_time - start_time;
        res.map_or_else(
            |e| {
                if e.is_timeout() {
                    Err(ErrorType::Timeout)
                } else {
                    Err(ErrorType::Unknown)
                }
            },
            |res| Ok((res, duration)),
        )
    }

    async fn verify_status_or_content(&self, res: reqwest::Response) -> Option<ErrorType> {
        if let Some(status) = self.ok_when.status {
            if res.status().as_u16() != status {
                return Some(ErrorType::WrongStatus);
            }
        }
        if let Some(content) = &self.ok_when.content {
            let body = res.text().await.unwrap_or_else(|e| {
                eprintln!("Error while reading response body: {}", e);
                String::new() // Check will fail because we search in an empty string
            });
            if !body.contains(content) {
                return Some(ErrorType::WrongContent);
            }
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Status {
    Online(Duration),
    Offline(ErrorType),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ErrorType {
    Timeout,
    WrongContent,
    WrongStatus,
    Unknown,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OkWhen {
    #[serde(default = "default_ok_status")]
    status: Option<u16>,
    content: Option<String>,
}

impl Default for OkWhen {
    fn default() -> Self {
        OkWhen {
            status: default_ok_status(),
            content: None,
        }
    }
}

const fn default_ok_status() -> Option<u16> {
    Some(200)
}
