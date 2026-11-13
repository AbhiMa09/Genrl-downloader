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
        let fetched = self
            .gate
            .get_keyed(breaker_key, &full_url)
            .await
            .map_err(|e| match e {
                crate::error::Error::BreakerOpen { service, .. } => {
                    IndexerError::BreakerOpen(service)
                }
                crate::error::Error::Timeout { .. } => IndexerError::Timeout,
                other => IndexerError::Http(other.to_string()),
            })?;
        String::from_utf8(fetched.body)
            .map_err(|e| IndexerError::Parse(format!("utf-8 conversion error: {e}")))
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
