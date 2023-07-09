/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize)]
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
            Ok((res, duration)) => match self.verify_status_or_dom(res).await {
                Some(err) => Status::Offline(err),
                None => Status::Online(duration),
            },
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
        match res {
            Ok(res) => Ok((res, duration)),
            Err(e) => {
                if e.is_timeout() {
                    Err(ErrorType::Timeout)
                } else {
                    Err(ErrorType::Unknown)
                }
            }
        }
    }

    async fn verify_status_or_dom(&self, res: reqwest::Response) -> Option<ErrorType> {
        if let Some(status) = self.ok_when.status {
            if res.status().as_u16() != status {
                return Some(ErrorType::WrongResponse);
            }
        }
        if let Some(dom) = &self.ok_when.dom {
            let body = res.text().await.unwrap_or_else(|e| {
                eprintln!("Error while reading response body: {}", e);
                String::new() // Check will fail because we search in an empty string
            });
            if !body.contains(dom) {
                return Some(ErrorType::WrongResponse);
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
    WrongResponse,
    Unknown,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OkWhen {
    #[serde(default = "default_ok_status")]
    status: Option<u16>,
    dom: Option<String>,
}

impl Default for OkWhen {
    fn default() -> Self {
        OkWhen {
            status: default_ok_status(),
            dom: None,
        }
    }
}

fn default_ok_status() -> Option<u16> {
    Some(200)
}
