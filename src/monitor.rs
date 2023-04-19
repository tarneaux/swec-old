use reqwest::Client;
use std::fmt::{Debug, Formatter};
use std::time::Duration;

pub struct ServiceWatcher {
    url: String,
    timeout: Duration,
    ok_when: OKWhen,
}

pub enum Status {
    Online(Duration),
    Offline,
}

impl Clone for Status {
    fn clone(&self) -> Self {
        match self {
            Status::Online(d) => Status::Online(*d),
            Status::Offline => Status::Offline,
        }
    }
}

impl Debug for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Online(d) => write!(f, "Online({:?})", d),
            Status::Offline => write!(f, "Offline"),
        }
    }
}

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
    pub async fn get_current_status(&mut self) -> Status {
        let res = self.get_url().await;
        match res {
            Some((res, duration)) => {
                let status = self.verify_status_or_dom(res).await;
                match &status {
                    Status::Online(_) => Status::Online(duration),
                    Status::Offline => Status::Offline,
                }
            }
            None => Status::Offline,
        }
    }

    async fn get_url(&self) -> Option<(reqwest::Response, Duration)> {
        let client = Client::new();
        let start_time = std::time::Instant::now();
        let res = client.get(&self.url).timeout(self.timeout).send().await;
        let end_time = std::time::Instant::now();
        let duration = end_time - start_time;
        match res {
            Ok(res) => Some((res, duration)),
            Err(e) => {
                println!("Error: {}", e);
                None
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
            Status::Offline
        }
    }

    async fn verify_dom(&self, res: reqwest::Response, wanted_dom: &str) -> Status {
        let body = res.text().await.unwrap();
        if body.contains(wanted_dom) {
            Status::Online(Duration::from_secs(0))
        } else {
            Status::Offline
        }
    }
}
