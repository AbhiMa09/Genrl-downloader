use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::types::IndexerError;
use crate::http::Gate;

/// HTTP fetching abstraction for indexers and metadata providers.
#[async_trait]
pub trait HttpFetch: Send + Sync {
    async fn get_text(
        &self,
        breaker_key: &str,
        url: &str,
        query_params: &[(&str, &str)],
    ) -> Result<String, IndexerError>;
}

/// Production implementation routing all calls through the existing Gate.
pub struct GateFetch {
    gate: Arc<Gate>,
}

impl GateFetch {
    pub fn new(gate: Arc<Gate>) -> Self {
        Self { gate }
    }
}

#[async_trait]
impl HttpFetch for GateFetch {
    async fn get_text(
        &self,
        breaker_key: &str,
        url: &str,
        query_params: &[(&str, &str)],
    ) -> Result<String, IndexerError> {
        let mut parsed = reqwest::Url::parse(url)
            .map_err(|e| IndexerError::Http(format!("invalid url {url}: {e}")))?;
        if !query_params.is_empty() {
            let mut pairs = parsed.query_pairs_mut();
            for (k, v) in query_params {
                pairs.append_pair(k, v);
            }
        }
        let full_url = parsed.to_string();
        match self.gate.get_keyed(breaker_key, &full_url).await {
            Ok(fetched) => String::from_utf8(fetched.body)
                .map_err(|e| IndexerError::Parse(format!("utf-8 conversion error: {e}"))),
            Err(crate::error::Error::Status {
                status,
                body: Some(body),
                ..
            }) if (400..=499).contains(&status) && !body.trim().is_empty() => {
                // Hand the response body back on 4xx so Torznab parser can inspect for <error> payload
                Ok(body)
            }
            Err(crate::error::Error::Status {
                status: 401 | 403, ..
            }) => Err(IndexerError::Http("unauthorized, check API key".into())),
            Err(crate::error::Error::BreakerOpen { service, .. }) => {
                Err(IndexerError::BreakerOpen(service))
            }
            Err(crate::error::Error::Timeout { .. }) => Err(IndexerError::Timeout),
            Err(other) => Err(IndexerError::Http(other.to_string())),
        }
    }
}

/// Fake HTTP fetch for deterministic unit and integration tests without network.
#[derive(Default)]
pub struct FakeFetch {
    responses: Mutex<HashMap<String, String>>,
    errors: Mutex<HashMap<String, String>>,
}

impl FakeFetch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_response(&self, url_prefix: &str, body: impl Into<String>) {
        self.responses
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(url_prefix.to_string(), body.into());
    }

    pub fn set_error(&self, url_prefix: &str, error: impl Into<String>) {
        self.errors
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(url_prefix.to_string(), error.into());
    }
}

#[async_trait]
impl HttpFetch for FakeFetch {
    async fn get_text(
        &self,
        _breaker_key: &str,
        url: &str,
        query_params: &[(&str, &str)],
    ) -> Result<String, IndexerError> {
        let mut full = url.to_string();
        if !query_params.is_empty() {
            let mut sep = if full.contains('?') { '&' } else { '?' };
            for (k, v) in query_params {
                full.push(sep);
                full.push_str(k);
                full.push('=');
                full.push_str(v);
                sep = '&';
            }
        }

        let errs = self.errors.lock().unwrap_or_else(|e| e.into_inner());
        for (pattern, err) in errs.iter() {
            if full.contains(pattern) {
                return Err(IndexerError::Http(err.clone()));
            }
        }

        let resps = self.responses.lock().unwrap_or_else(|e| e.into_inner());
        for (pattern, body) in resps.iter() {
            if full.contains(pattern) {
                return Ok(body.clone());
            }
        }

        Err(IndexerError::Http(format!(
            "FakeFetch: unmocked URL {full}"
        )))
    }
}

impl FakeFetch {
    /// Helper to mock HTTP responses according to CHECK 3 rules:
    /// - 200 with body -> Ok(body)
    /// - 4xx with non-empty body -> Ok(body) (lets Torznab parser check for <error>)
    /// - 401/403 with empty body -> Err("unauthorized, check API key")
    pub fn set_status_response(&self, url_prefix: &str, status: u16, body: &str) {
        if (400..=499).contains(&status) {
            if body.trim().is_empty() {
                if status == 401 || status == 403 {
                    self.set_error(url_prefix, "unauthorized, check API key");
                } else {
                    self.set_error(url_prefix, format!("http status {status}"));
                }
            } else {
                self.set_response(url_prefix, body);
            }
        } else if (200..=299).contains(&status) {
            self.set_response(url_prefix, body);
        } else {
            self.set_error(url_prefix, format!("http status {status}"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_200_with_error_body() {
        let fetch = FakeFetch::new();
        fetch.set_status_response(
            "http://localhost:9696",
            200,
            r#"<error code="100" description="Invalid API Key"/>"#,
        );
        let res = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect("must hand body back on 200");
        assert!(res.contains("Invalid API Key"));
    }

    #[tokio::test]
    async fn test_fetch_401_with_error_body() {
        let fetch = FakeFetch::new();
        fetch.set_status_response(
            "http://localhost:9696",
            401,
            r#"<error code="100" description="Unauthorized access"/>"#,
        );
        let res = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect("must hand body back on 401 with XML body");
        assert!(res.contains("Unauthorized access"));
    }

    #[tokio::test]
    async fn test_fetch_401_with_empty_body() {
        let fetch = FakeFetch::new();
        fetch.set_status_response("http://localhost:9696", 401, "");
        let err = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect_err("must error on 401 with empty body");
        assert_eq!(err.to_string(), "http error: unauthorized, check API key");
    }
}
