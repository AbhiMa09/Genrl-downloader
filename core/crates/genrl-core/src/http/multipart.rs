use std::time::Instant;

use super::gate::Gate;
use super::policy::Service;
use super::types::{Fetched, GetOptions};
use super::wire::read_body;
use crate::error::{Error, Result};
use crate::indexer::redact_secrets;

impl Gate {
    /// POST multipart form data through the gate with circuit breaker accounting,
    /// rate limiting, timeout, size cap, and error redaction.
    pub async fn post_multipart(
        &self,
        service: Service,
        url: &str,
        fields: &[(String, String)],
        file_part: (String, String, Vec<u8>), // (field_name, filename, bytes)
        opts: &GetOptions,
    ) -> Result<Fetched> {
        let parsed =
            reqwest::Url::parse(url).map_err(|_| Error::InvalidUrl(redact_secrets(url)))?;
        let state = self.state(service);
        self.allow_str(service.as_str(), &state)?;

        let timeout = opts.timeout.unwrap_or(state.policy.timeout);
        let cap = state.policy.max_body_bytes;

        // Rate limiting
        state.limiter.acquire().await;

        let (field_name, filename, bytes) = file_part;
        let mut form = reqwest::multipart::Form::new();
        for (name, val) in fields {
            form = form.text(name.clone(), val.clone());
        }
        let part = reqwest::multipart::Part::bytes(bytes).file_name(filename);
        form = form.part(field_name, part);

        let mut req = self
            .client
            .post(parsed.clone())
            .multipart(form)
            .timeout(timeout);
        for (name, value) in &opts.headers {
            req = req.header(name.as_str(), value.as_str());
        }

        let resp = match req.send().await {
            Ok(resp) => resp,
            Err(err) => {
                state
                    .breaker
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_failure(Instant::now());
                return if err.is_timeout() {
                    Err(Error::Timeout {
                        url: redact_secrets(url),
                        timeout_ms: timeout.as_millis() as u64,
                    })
                } else {
                    Err(Error::Http(err))
                };
            }
        };

        let status = resp.status().as_u16();
        let set_cookie = resp
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let body = read_body(resp, cap, url, timeout).await?;

        if (200..=299).contains(&status) {
            state
                .breaker
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .on_success();
            Ok(Fetched {
                url: redact_secrets(url),
                status,
                body,
                from_cache: false,
                etag: None,
                set_cookie,
            })
        } else {
            // A 403 (e.g. qBittorrent session expired) is a client error, not a service outage,
            // so upstream can handle re-login without immediately tripping the breaker on the first attempt.
            // But 429 and 5xx count as failures for breaker accounting.
            if status == 429 || status >= 500 {
                state
                    .breaker
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .on_failure(Instant::now());
            }
            Err(Error::Status {
                status,
                url: redact_secrets(url),
                retry_after: None,
                body: Some(redact_secrets(&String::from_utf8_lossy(&body))),
            })
        }
    }
}
