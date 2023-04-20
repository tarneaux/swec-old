use reqwest::Client;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

#[derive(Clone)]
pub struct ServiceWatcher {
    url: String,
    timeout: Duration,
    ok_when: OKWhen,
}

pub enum Status {
    Online(Duration),
    Offline(ErrorType),
}

impl Debug for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Online(d) => write!(f, "Online({:?})", d),
            Status::Offline(e) => write!(f, "Offline: {:?}", e),
        }
    }
}

pub enum ErrorType {
    Timeout,
    WrongStatus,
    WrongDom,
    Unknown(String),
}

impl Debug for ErrorType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorType::Timeout => write!(f, "Timeout"),
            ErrorType::WrongStatus => write!(f, "Wrong status code"),
            ErrorType::WrongDom => write!(f, "Wrong dom"),
            ErrorType::Unknown(e) => write!(f, "Unknown error from reqwest (see https://dtantsur.github.io/rust-openstack/reqwest/struct.Error.html): {}", e),
        }
    }
}

#[derive(Clone)]
pub enum OKWhen {
    Status(u16),
    InDom(String),
}

impl ServiceWatcher {
    pub fn new(url: &str, timeout: Duration, wanted_status: OKWhen) -> Self {
        ServiceWatcher {
            url: url.to_string(),
            timeout,
            ok_when: wanted_status,
        }
    }
    pub async fn get_current_status(&self) -> Status {
        let res = self.get_url().await;
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

    async fn get_url(&self) -> Result<(reqwest::Response, Duration), ErrorType> {
        let client = Client::new();
        let start_time = std::time::Instant::now();
        let res = client.get(&self.url).timeout(self.timeout).send().await;
        let end_time = std::time::Instant::now();
        let duration = end_time - start_time;
        match res {
            Ok(res) => Ok((res, duration)),
            Err(e) => {
                if e.is_timeout() {
                    Err(ErrorType::Timeout)
                } else {
                    Err(ErrorType::Unknown(e.to_string()))
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
            Status::Offline(ErrorType::WrongStatus)
        }
    }

    async fn verify_dom(&self, res: reqwest::Response, wanted_dom: &str) -> Status {
        let body = res.text().await.unwrap();
        if body.contains(wanted_dom) {
            Status::Online(Duration::from_secs(0))
        } else {
            Status::Offline(ErrorType::WrongDom)
        }
    }
}
