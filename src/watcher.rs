/*
 * Swec: Simple Web Endpoint Checker
 * Author: tarneo <tarneo@tarneo.fr>
 * License: GPLv2
 */

use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
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
                .map_or_else(|| Status::Up(duration), Status::Down),
            Err(e) => Status::Down(e),
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
        let body = res.text().await.unwrap_or_else(|e| {
            eprintln!("Error while reading response body: {}", e);
            String::new() // Check will fail because we search in an empty string
        });
        if let Some(content) = &self.ok_when.content {
            if !body.contains(content) {
                return Some(ErrorType::WrongContent);
            }
        }
        if !self.ok_when.content_regex.is_match(&body) {
            return Some(ErrorType::WrongContent);
        }
        None
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Status {
    Up(Duration),
    Down(ErrorType),
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ErrorType {
    Timeout,
    WrongContent,
    WrongStatus,
    Unknown,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OkWhen {
    #[serde(default = "default_ok_status")]
    status: Option<u16>,
    content: Option<String>,
    #[serde(
        serialize_with = "regex_serialize",
        deserialize_with = "regex_deserialize",
        default = "default_ok_regex"
    )]
    content_regex: Regex,
}

impl PartialEq for OkWhen {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status
            && self.content == other.content
            && self.content_regex.as_str() == other.content_regex.as_str()
    }
}

impl Default for OkWhen {
    fn default() -> Self {
        Self {
            status: default_ok_status(),
            content: None,
            content_regex: default_ok_regex(),
        }
    }
}

const fn default_ok_status() -> Option<u16> {
    Some(200)
}

fn default_ok_regex() -> Regex {
    Regex::new("").unwrap()
}

fn regex_serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(regex.as_str())
}

fn regex_deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Regex::new(&s).map_err(serde::de::Error::custom)
}
