use std::time::Duration;

use crate::error::Result;
use super::policy::RetryPolicy;

/// Options for a single request. `Default` is "use the service policy".
#[derive(Debug, Clone, Default)]
pub struct GetOptions {
    /// Bypass the cache (still writes the response afterwards).
    pub skip_cache: bool,
    /// Override the cache lifetime for this call.
    pub ttl: Option<Duration>,
    /// Override the per-request timeout.
    pub timeout: Option<Duration>,
    /// Override the retry/backoff policy.
    pub retry: Option<RetryPolicy>,
    pub headers: Vec<(String, String)>,
}

/// A response that has passed every gate.
#[derive(Debug, Clone)]
pub struct Fetched {
    pub url: String,
    pub status: u16,
    pub body: Vec<u8>,
    pub from_cache: bool,
    pub etag: Option<String>,
    /// First `Set-Cookie` value, for services that hand out a session cookie
    /// (qBittorrent's `SID`). Never present on a cached response.
    pub set_cookie: Option<String>,
}

impl Fetched {
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|_| crate::Error::Other(format!("non-utf8 response from {}", self.url)))
    }

    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_slice(&self.body)?)
    }

    pub fn len(&self) -> usize {
        self.body.len()
    }

    pub fn is_empty(&self) -> bool {
        self.body.is_empty()
    }
}
