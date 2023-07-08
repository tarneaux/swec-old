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
    pub ok_when: OkWhen,
    pub name: String,
}

impl ServiceWatcher {
    pub async fn get_current_status(&self, timeout: &Duration) -> Status {
        let res = self.get_url(timeout).await;
        match res {
            Ok((res, duration)) => {
                let mut status = self.verify_status_or_dom(res).await;
                if let Status::Online(_) = status {
                    status = Status::Online(duration);
                }
                status
            }
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

    async fn verify_status_or_dom(&self, res: reqwest::Response) -> Status {
        match &self.ok_when {
            OkWhen::Status(status) => self.verify_status_code(res, *status).await,
            OkWhen::InDom(dom) => {
                let dom = dom.to_string();
                self.verify_dom(res, &dom).await
            }
        }
    }

    async fn verify_status_code(&self, res: reqwest::Response, wanted_status_code: u16) -> Status {
        if res.status().as_u16() == wanted_status_code {
            Status::Online(Duration::from_secs(0))
        } else {
            Status::Offline(ErrorType::WrongResponse)
        }
    }

    async fn verify_dom(&self, res: reqwest::Response, wanted_dom: &str) -> Status {
        let body = res.text().await.unwrap_or_else(|e| {
            eprintln!("Error while reading body of response: {}", e);
            "".to_string()
        });
        if body.contains(wanted_dom) {
            Status::Online(Duration::from_secs(0))
        } else {
            Status::Offline(ErrorType::WrongResponse)
        }
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
pub enum OkWhen {
    Status(u16),
    InDom(String),
}
