use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ServiceWatcher {
    pub url: String,
    pub ok_when: OKWhen,
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
            OKWhen::Status(status) => self.verify_status_code(res, *status).await,
            OKWhen::InDom(dom) => {
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
        let body = res.text().await.unwrap();
        if body.contains(wanted_dom) {
            Status::Online(Duration::from_secs(0))
        } else {
            Status::Offline(ErrorType::WrongResponse)
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub enum Status {
    Online(Duration),
    Offline(ErrorType),
}

impl Debug for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Online(d) => write!(f, "Online({:?})", d),
            Self::Offline(e) => write!(f, "Offline: {:?}", e),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub enum ErrorType {
    Timeout,
    WrongResponse,
    Unknown,
}

impl Debug for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "Timeout"),
            Self::WrongResponse => write!(f, "Wrong response: didn't match OKWhen"),
            // Other errors from reqwest, see https://dtantsur.github.io/rust-openstack/reqwest/struct.Error.html
            Self::Unknown => write!(f, "Unknown error"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum OKWhen {
    Status(u16),
    InDom(String),
}
