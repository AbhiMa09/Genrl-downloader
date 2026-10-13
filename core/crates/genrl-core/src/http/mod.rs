//! The shared HTTP client layer.
//!
//! No module talks to the network directly: they ask a [`Gate`] for one of the
//! five allowed [`Service`]s. The gate applies, in order:
//!
//! 1. the circuit breaker (five consecutive failures open it for 15 minutes),
//! 2. the response cache (fresh entries answer without a request, stale ETags
//!    revalidate with `If-None-Match`),
//! 3. the token-bucket limiter (Jikan 2/s + 60/min, Nyaa 1 per 2 s, …),
//! 4. the request itself, with a size cap and a timeout,
//! 5. retry with the backoff rule — 5 s, 30 s, 2 min, 10 min, 1 hour with
//!    jitter, or `Retry-After` when the server sends one.

pub mod backoff;
pub mod breaker;
pub mod cache;
pub mod limiter;
pub mod policy;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub use policy::{RetryPolicy, Service, ServicePolicy};

pub use breaker::State as BreakerState;
pub use policy::ServicePolicy as Policy;

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

struct ServiceState {
    policy: ServicePolicy,
    limiter: limiter::Limiter,
    breaker: Mutex<breaker::CircuitBreaker>,
}

pub struct Gate {
    client: reqwest::Client,
    db: crate::Db,
    states: Mutex<HashMap<Service, Arc<ServiceState>>>,
}

impl Gate {
    /// Build a gate with the design's default policies for all five services.
    pub fn new(db: crate::Db) -> Result<Self> {
        Self::with_policies(db, &[])
    }

    /// Build a gate, replacing any policies that are supplied (tests use this
    /// to shorten timeouts and backoff steps).
    pub fn with_policies(db: crate::Db, overrides: &[(Service, ServicePolicy)]) -> Result<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(3))
            .user_agent(concat!("GenrlDownloader/", env!("CARGO_PKG_VERSION")))
            .build()?;

        let mut states = HashMap::new();
        for service in Service::ALL {
            let policy = overrides
                .iter()
                .find(|(s, _)| *s == service)
                .map(|(_, p)| p.clone())
                .unwrap_or_else(|| ServicePolicy::for_service(service));
            let limiter = limiter::Limiter::new(&policy.rate);
            let breaker =
                breaker::CircuitBreaker::new(policy.breaker_threshold, policy.breaker_cooldown);
            states.insert(
                service,
                Arc::new(ServiceState {
                    policy,
                    limiter,
                    breaker: Mutex::new(breaker),
                }),
            );
        }

        Ok(Gate {
            client,
            db,
            states: Mutex::new(states),
        })
    }

    pub fn db(&self) -> &crate::Db {
        &self.db
    }

    fn state(&self, service: Service) -> Arc<ServiceState> {
        let map = self.states.lock().unwrap_or_else(|e| e.into_inner());
        map.get(&service)
            .cloned()
            .expect("every service has a state")
    }

    pub fn policy(&self, service: Service) -> ServicePolicy {
        self.state(service).policy.clone()
    }

    /// Current breaker state for a service (the status strip shows this).
    pub fn breaker_state(&self, service: Service) -> BreakerState {
        let state = self.state(service);
        let guard = state.breaker.lock().unwrap_or_else(|e| e.into_inner());
        guard.state()
    }

    /// Would a request for `service` be allowed right now?
    ///
    /// Non-consuming: it never claims the half-open probe, so the queue can ask
    /// this on every tick before resuming a job blocked on that service.
    pub fn would_allow(&self, service: Service) -> bool {
        let state = self.state(service);
        let mut guard = state.breaker.lock().unwrap_or_else(|e| e.into_inner());
        guard.peek(Instant::now())
    }

    /// How long a fresh caller would wait on the limiter right now.
    pub fn rate_wait(&self, service: Service) -> Duration {
        self.state(service).limiter.queued_wait()
    }

    /// GET a URL through the gate.
    pub async fn get(&self, service: Service, url: &str) -> Result<Fetched> {
        self.get_with(service, url, &GetOptions::default()).await
    }

    /// GET with per-call overrides.
    pub async fn get_with(
        &self,
        service: Service,
        url: &str,
        opts: &GetOptions,
    ) -> Result<Fetched> {
        let parsed = reqwest::Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_string()))?;
        let key = parsed.to_string();
        let state = self.state(service);

        // 1. Circuit breaker: jobs wait in `blocked` instead of hammering.
        self.allow(service, &state)?;

        let ttl = opts.ttl.or(state.policy.cache_ttl);

        // 2. Cache: fresh answers immediately, stale ETags revalidate.
        let mut if_none_match: Option<String> = None;
        if ttl.filter(|_| !opts.skip_cache).is_some() {
            if let Some(entry) = cache::get(&self.db, service.as_str(), &key)? {
                if entry.is_fresh(crate::db::now_unix()) {
                    return Ok(Fetched {
                        url: key,
                        status: 200,
                        body: entry.body,
                        from_cache: true,
                        etag: entry.etag,
                        set_cookie: None,
                    });
                }
                if_none_match = entry.etag;
            }
        }

        let wire = Wire::Get {
            if_none_match: if_none_match.as_deref(),
        };
        self.dispatch(service, parsed, key, state, opts, wire, ttl)
            .await
    }

    /// POST an `application/x-www-form-urlencoded` body through the gate.
    ///
    /// Commands (qBittorrent's WebUI is form POSTs) are never cached: the
    /// breaker, the limiter and the backoff still apply, the cache does not.
    pub async fn post_form(
        &self,
        service: Service,
        url: &str,
        form: &[(String, String)],
        opts: &GetOptions,
    ) -> Result<Fetched> {
        let parsed = reqwest::Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_string()))?;
        let key = parsed.to_string();
        let state = self.state(service);
        self.allow(service, &state)?;
        let wire = Wire::Form { fields: form };
        self.dispatch(service, parsed, key, state, opts, wire, None)
            .await
    }

    // `last_err` starts empty and is only read after an attempt has filled it.
    #[allow(unused_assignments)]
    async fn dispatch(
        &self,
        service: Service,
        parsed: reqwest::Url,
        key: String,
        state: Arc<ServiceState>,
        opts: &GetOptions,
        wire: Wire<'_>,
        ttl: Option<Duration>,
    ) -> Result<Fetched> {
        let timeout = opts.timeout.unwrap_or(state.policy.timeout);
        let retry = opts
            .retry
            .clone()
            .unwrap_or_else(|| state.policy.retry.clone());
        let cap = state.policy.max_body_bytes;

        let started = Instant::now();
        let mut attempt: u32 = 0;
        let mut last_err: Option<Error> = None;

        loop {
            attempt += 1;

            // 3. Rate limit: we always wait our turn rather than getting banned.
            state.limiter.acquire().await;
            // We already hold the breaker's permission (including a half-open
            // probe); only bail if another job hard-opened it while we waited.
            if self.breaker_state(service) == BreakerState::Open {
                return Err(Error::BreakerOpen {
                    service: service.as_str().to_string(),
                    retry_in_secs: state.policy.breaker_cooldown.as_secs(),
                });
            }

            let retry_after = match self
                .send(&parsed, &wire, opts, timeout, cap)
                .await
            {
                Ok(Outcome::Success {
                    status,
                    body,
                    etag,
                    set_cookie,
                }) => {
                    self.record_success(&state);
                    if status == 304 {
                        // Server confirmed our copy: refresh the clock, no body.
                        if let Some(ttl) = ttl {
                            cache::touch(&self.db, service.as_str(), &key, ttl.as_secs())?;
                            if let Some(entry) = cache::get(&self.db, service.as_str(), &key)? {
                                return Ok(Fetched {
                                    url: key,
                                    status: 304,
                                    body: entry.body,
                                    from_cache: true,
                                    etag,
                                    set_cookie: None,
                                });
                            }
                        }
                        return Ok(Fetched {
                            url: key,
                            status,
                            body,
                            from_cache: false,
                            etag,
                            set_cookie,
                        });
                    }
                    if let Some(ttl) = ttl {
                        cache::put(
                            &self.db,
                            service.as_str(),
                            &key,
                            &body,
                            etag.as_deref(),
                            ttl.as_secs(),
                        )?;
                    }
                    return Ok(Fetched {
                        url: key,
                        status,
                        body,
                        from_cache: false,
                        etag,
                        set_cookie,
                    });
                }
                Ok(Outcome::RateLimited { retry_after }) => {
                    last_err = Some(Error::RateLimited {
                        service: service.as_str().to_string(),
                        retry_after,
                    });
                    self.record_failure(&state);
                    retry_after
                }                Ok(Outcome::ServerError { status }) => {
                    last_err = Some(Error::Status {
                        status,
                        url: key.clone(),
                        retry_after: None,
                    });
                    self.record_failure(&state);
                    None
                }
                Ok(Outcome::ClientError { status }) => {
                    // 4xx is our problem, not the service being down: no retry,
                    // no breaker. (qBittorrent 403 triggers a re-login upstream.)
                    self.record_success(&state);
                    return Err(Error::Status {
                        status,
                        url: key.clone(),
                        retry_after: None,
                    });
                }
                Err(err) => {
                    // A hostile or broken body is not worth a second attempt,
                    // and says nothing about the service being healthy.
                    if matches!(err, Error::BodyTooLarge { .. }) {
                        self.record_success(&state);
                        return Err(err);
                    }
                    last_err = Some(err);
                    self.record_failure(&state);
                    None
                }
            };

            // Five consecutive failures: open the breaker now.
            if self.breaker_state(service) == BreakerState::Open {
                return Err(Error::BreakerOpen {
                    service: service.as_str().to_string(),
                    retry_in_secs: state.policy.breaker_cooldown.as_secs(),
                });
            }

            if attempt >= retry.max_attempts {
                return Err(last_err.unwrap_or_else(|| Error::Other("retry exhausted".into())));
            }

            let delay = match retry_after {
                // A 429 with Retry-After uses that value instead of the schedule.
                Some(d) => d.min(retry.max_delay),
                None => {
                    backoff::jitter(backoff::delay_for(attempt, &retry.schedule))
                        .min(retry.max_delay)
                }
            };

            if started.elapsed() + delay > retry.max_total_wait {
                // The job queue owns the rest of the schedule.
                return Err(last_err.unwrap_or_else(|| Error::Other("retry budget spent".into())));
            }
            tokio::time::sleep(delay).await;
        }
    }

    fn allow(&self, service: Service, state: &Arc<ServiceState>) -> Result<()> {
        use breaker::Verdict;
        let mut guard = state.breaker.lock().unwrap_or_else(|e| e.into_inner());
        match guard.check(Instant::now()) {
            Verdict::Allow => Ok(()),
            Verdict::Reject { retry_in } => Err(Error::BreakerOpen {
                service: service.as_str().to_string(),
                retry_in_secs: retry_in.as_secs().max(1),
            }),
        }
    }

    fn record_success(&self, state: &Arc<ServiceState>) {
        state
            .breaker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .on_success();
    }

    fn record_failure(&self, state: &Arc<ServiceState>) {
        state
            .breaker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .on_failure(Instant::now());
    }

    /// One request: timeout, size cap, status classification.
    async fn send(
        &self,
        url: &reqwest::Url,
        wire: &Wire<'_>,
        opts: &GetOptions,
        timeout: Duration,
        cap: usize,
    ) -> std::result::Result<Outcome, Error> {
        let mut req = match wire {
            Wire::Get { if_none_match } => {
                let mut req = self.client.get(url.clone());
                if let Some(tag) = if_none_match {
                    req = req.header(reqwest::header::IF_NONE_MATCH, *tag);
                }
                req
            }
            Wire::Form { fields } => self.client.post(url.clone()).form(*fields),
        };
        req = req.timeout(timeout);
        for (name, value) in &opts.headers {
            req = req.header(name.as_str(), value.as_str());
        }

        let timeout_ms = timeout.as_millis() as u64;
        let resp = match req.send().await {
            Ok(resp) => resp,
            Err(err) => {
                return if err.is_timeout() {
                    Err(Error::Timeout {
                        url: url.to_string(),
                        timeout_ms,
                    })
                } else {
                    Err(Error::Http(err))
                };
            }
        };

        let status = resp.status().as_u16();
        let etag = resp
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let set_cookie = resp
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        match status {
            200..=299 => {
                let body = read_body(resp, cap, url.as_str(), timeout).await?;
                Ok(Outcome::Success {
                    status,
                    body,
                    etag,
                    set_cookie,
                })
            }
            304 => Ok(Outcome::Success {
                status,
                body: Vec::new(),
                etag,
                set_cookie,
            }),
            429 => {
                let header = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok());
                let retry_after = backoff::parse_retry_after(header);
                Ok(Outcome::RateLimited { retry_after })
            }
            408 | 425 | 500..=599 => Ok(Outcome::ServerError { status }),
            _ => Ok(Outcome::ClientError { status }),
        }
    }
}

/// What one in-flight request actually does on the wire.
enum Wire<'a> {
    Get {
        if_none_match: Option<&'a str>,
    },
    Form {
        fields: &'a [(String, String)],
    },
}

enum Outcome {
    Success {
        status: u16,
        body: Vec<u8>,
        etag: Option<String>,
        set_cookie: Option<String>,
    },
    RateLimited {
        retry_after: Option<Duration>,
    },
    ServerError {
        status: u16,
    },
    ClientError {
        status: u16,
    },
}

async fn read_body(
    resp: reqwest::Response,
    cap: usize,
    url: &str,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut resp = resp;
    let mut body: Vec<u8> = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                let total = body.len() + chunk.len();
                if total > cap {
                    return Err(Error::BodyTooLarge {
                        url: url.to_string(),
                        got: total,
                        cap,
                    });
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => break,
            Err(err) => {
                return if err.is_timeout() {
                    Err(Error::Timeout {
                        url: url.to_string(),
                        timeout_ms: timeout.as_millis() as u64,
                    })
                } else {
                    Err(Error::Http(err))
                };
            }
        }
    }
    Ok(body)
}

/// Only `magnet:?xt=urn:btih:` and `https://` `.torrent` links are accepted
/// from a feed. Private ranges and localhost are refused (SSRF), redirects are
/// capped at three by the client, and TLS is always verified.
pub fn is_acceptable_link(link: &str) -> bool {
    if let Some(rest) = link.strip_prefix("magnet:") {
        return rest.starts_with("?xt=urn:btih:") || rest.starts_with("xt=urn:btih:");
    }
    match reqwest::Url::parse(link) {
        // Private ranges are checked first so a "safe" scheme cannot smuggle
        // an internal address past us.
        Ok(url) => !host_is_private(&url) && url.scheme() == "https",
        Err(_) => false,
    }
}

/// Lexical private-address check for link validation.
fn host_is_private(url: &reqwest::Url) -> bool {
    let host = match url.host_str() {
        Some(h) => h.to_ascii_lowercase(),
        None => return true,
    };
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    let octets: Vec<&str> = host.split('.').collect();
    if octets.len() == 4 && octets.iter().all(|p| p.parse::<u8>().is_ok()) {
        let a = octets[0].parse::<u8>().unwrap();
        let b = octets[1].parse::<u8>().unwrap();
        return match a {
            0 | 10 | 127 => true,
            100 => (64..=127).contains(&b), // CGNAT
            169 => b == 254,                // link-local
            172 => (16..=31).contains(&b),
            192 => b == 168 || b == 0,
            198 => b == 18 || b == 19,
            224..=255 => true, // multicast / reserved
            _ => false,
        };
    }
    if host.starts_with('[') || host.contains(':') {
        let h = host.trim_start_matches('[').trim_end_matches(']');
        return h == "::1" || h == "::" || h.starts_with("fe80") || h.starts_with("fc") || h.starts_with("fd");
    }
    false
}

// Keeps the error type nameable inside this module without importing it twice.
use crate::error::{Error, Result};

#[cfg(test)]
mod tests {
    use super::policy::RateSpec;
    use super::*;

    #[test]
    fn accepts_magnets_and_public_torrent_links() {
        assert!(is_acceptable_link(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x"
        ));
        assert!(is_acceptable_link("https://nyaa.si/download/123.torrent"));
        assert!(!is_acceptable_link("ftp://example.com/x.torrent"));
        assert!(!is_acceptable_link("file:///etc/passwd"));
        assert!(!is_acceptable_link("http://nyaa.si/download/123.torrent"), "https only");
    }

    #[test]
    fn refuses_private_and_loopback_targets() {
        assert!(!is_acceptable_link("http://127.0.0.1:8080/x.torrent"));
        assert!(!is_acceptable_link("https://192.168.1.10/x.torrent"));
        assert!(!is_acceptable_link("https://10.0.0.1/x.torrent"));
        assert!(!is_acceptable_link("https://localhost/x.torrent"));
        assert!(!is_acceptable_link("http://169.254.169.254/latest/meta-data"));
        assert!(!is_acceptable_link("not a url"));
    }

    #[test]
    fn rate_specs_match_the_design() {
        let jikan = ServicePolicy::for_service(Service::Jikan);
        assert_eq!(jikan.rate.permits_per_second, 2.0);
        assert_eq!(jikan.rate.per_minute_cap, Some(60));
        assert_eq!(jikan.cache_ttl, Some(Duration::from_secs(7 * 24 * 3600)));

        let nyaa = ServicePolicy::for_service(Service::Nyaa);
        assert_eq!(nyaa.rate.permits_per_second, 0.5, "one request per 2 seconds");
        assert!(nyaa.rate.jitter);
        assert_eq!(nyaa.cache_ttl, Some(Duration::from_secs(600)));

        for service in Service::ALL {
            let p = ServicePolicy::for_service(service);
            assert_eq!(p.breaker_threshold, 5, "{service}");
            assert_eq!(p.breaker_cooldown, Duration::from_secs(900), "{service}");
            assert!(p.max_body_bytes >= 5 * 1024 * 1024, "{service}");
        }

        let spec = RateSpec {
            permits_per_second: 2.0,
            burst: 2,
            per_minute_cap: Some(60),
            jitter: false,
        };
        assert_eq!(spec.per_minute_cap, Some(60));
    }
}
