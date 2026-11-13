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
            }) => {
                let trimmed = body.trim_start();
                let is_xml = trimmed.starts_with("<?xml")
                    || trimmed.starts_with("<error")
                    || trimmed.starts_with("<rss");
                let lower = trimmed.to_ascii_lowercase();
                let is_html = lower.starts_with("<!doctype html")
                    || lower.starts_with("<html")
                    || lower.contains("cloudflare")
                    || lower.contains("captcha");

                if (status == 400 || status == 401 || status == 403) && is_xml {
                    // Only 400, 401, 403 with XML return Ok(body) so Torznab parser can inspect <error>
                    Ok(body)
                } else if (status == 403 || status == 503) && is_html {
                    Err(IndexerError::Http(
                        "indexer blocked by Cloudflare or captcha, check Prowlarr/FlareSolverr"
                            .into(),
                    ))
                } else if status == 401 || status == 403 {
                    Err(IndexerError::Http("unauthorized, check API key".into()))
                } else {
                    Err(IndexerError::Http(format!("http status {status}")))
                }
            }
            Err(crate::error::Error::Status { status, .. }) => {
                if status == 401 || status == 403 {
                    Err(IndexerError::Http("unauthorized, check API key".into()))
                } else {
                    Err(IndexerError::Http(format!("http status {status}")))
                }
            }
            Err(crate::error::Error::RateLimited { retry_after, .. }) => Err(IndexerError::Http(
                format!("rate limited (retry after {:?})", retry_after),
            )),
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
    /// Helper to mock HTTP responses according to ADD 1 rules:
    /// - 400/401/403 with XML -> Ok(body) (lets Torznab parser check for <error>)
    /// - 403/503 with HTML -> Err("indexer blocked by Cloudflare or captcha...")
    /// - 401/403 without XML -> Err("unauthorized, check API key")
    /// - 200..=299 -> Ok(body)
    /// - Other 4xx/5xx -> Err
    pub fn set_status_response(&self, url_prefix: &str, status: u16, body: &str) {
        let trimmed = body.trim_start();
        let is_xml = trimmed.starts_with("<?xml")
            || trimmed.starts_with("<error")
            || trimmed.starts_with("<rss");
        let lower = trimmed.to_ascii_lowercase();
        let is_html = lower.starts_with("<!doctype html")
            || lower.starts_with("<html")
            || lower.contains("cloudflare")
            || lower.contains("captcha");

        if (status == 400 || status == 401 || status == 403) && is_xml {
            self.set_response(url_prefix, body);
        } else if (status == 403 || status == 503) && is_html {
            self.set_error(
                url_prefix,
                "indexer blocked by Cloudflare or captcha, check Prowlarr/FlareSolverr",
            );
        } else if status == 401 || status == 403 {
            self.set_error(url_prefix, "unauthorized, check API key");
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
    async fn test_fetch_400_with_xml_error_body_returns_ok() {
        let fetch = FakeFetch::new();
        fetch.set_status_response(
            "http://localhost:9696",
            400,
            r#"<?xml version="1.0"?><error code="100" description="Invalid API Key"/>"#,
        );
        let res = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect("must return Ok(body) on 400 with XML error payload");
        assert!(res.contains("Invalid API Key"));
    }

    #[tokio::test]
    async fn test_fetch_403_with_html_returns_cloudflare_error() {
        let fetch = FakeFetch::new();
        fetch.set_status_response(
            "http://localhost:9696",
            403,
            "<!DOCTYPE html><html><title>Just a moment... Cloudflare</title></html>",
        );
        let err = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect_err("must error on 403 with HTML");
        assert!(err
            .to_string()
            .contains("indexer blocked by Cloudflare or captcha, check Prowlarr/FlareSolverr"));
    }

    #[tokio::test]
    async fn test_fetch_503_with_body_returns_err() {
        let fetch = FakeFetch::new();
        fetch.set_status_response("http://localhost:9696", 503, "Service Unavailable");
        let err = fetch
            .get_text("torznab:1", "http://localhost:9696", &[])
            .await
            .expect_err("503 must return Err");
        assert!(err.to_string().contains("503"));
    }

    #[tokio::test]
    async fn test_gate_fetch_429_returns_err_and_trips_breaker() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
            .mount(&server)
            .await;

        let db = crate::db::Db::open_in_memory().unwrap();
        let gate = Arc::new(crate::http::Gate::new(db).unwrap());
        let fetch = GateFetch::new(gate.clone());

        let err = fetch
            .get_text("torznab:trip_test", &server.uri(), &[])
            .await
            .expect_err("429 must return Err");
        assert!(err.to_string().contains("rate limited"));
        // Confirm breaker accounting recorded the failure
        assert!(
            gate.dynamic_breaker_failures("torznab:trip_test") > 0,
            "429 must record failure in dynamic breaker"
        );
    }
}
