use std::thread;
use std::time::{Duration, Instant};

pub struct Watcher {
    url: String,
    normal_status: u16,
    max_health_checks: usize,
    check_timeout: Duration,
    check_interval: Duration,
    health_checks: Vec<HealthCheck>,
}

impl Watcher {
    pub fn new(
        url: String,
        normal_status: u16,
        max_health_checks: u32,
        check_timeout: Duration,
        check_interval: Duration,
    ) -> Watcher {
        Watcher {
            url,
            normal_status,
            max_health_checks: max_health_checks as usize,
            check_timeout,
            check_interval,
            health_checks: Vec::new(),
        }
    }

    pub async fn check_health(&mut self) {
        let health_check = self.get_status().await;
        if self.health_checks.len() >= self.max_health_checks {
            self.health_checks.remove(0);
        }
        self.health_checks.push(health_check);
    }

    async fn get_status(&self) -> HealthCheck {
        let start = Instant::now();
        let session = reqwest::Client::new();
        let response = session
            .get(&self.url)
            .timeout(self.check_timeout)
            .send()
            .await;
        let ping = start.elapsed().as_millis() as u32;
        let is_online = match response {
            Ok(response) => response.status().as_u16() == self.normal_status,
            Err(_) => false,
        };
        let ping = if is_online { Some(ping) } else { None };
        HealthCheck { is_online, ping }
    }

    pub fn get_last_check(&self) -> Option<&HealthCheck> {
        self.health_checks.last()
    }
}

pub struct HealthCheck {
    is_online: bool,
    ping: Option<u32>,
}

impl HealthCheck {
    pub fn get_ping(&self) -> Option<u32> {
        self.ping
    }

    pub fn is_online(&self) -> bool {
        self.is_online
    }
}
