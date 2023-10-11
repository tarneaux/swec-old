use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct OkWhen {
    #[serde(default = "default_ok_status")]
    pub status: Option<u16>,
    pub content: Option<String>,
    #[serde(
        serialize_with = "regex_serialize",
        deserialize_with = "regex_deserialize",
        default = "default_ok_regex"
    )]
    pub content_regex: Regex,
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
