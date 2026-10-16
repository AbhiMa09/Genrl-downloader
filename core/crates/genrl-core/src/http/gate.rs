use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::backoff;
use super::breaker::{self, State as BreakerState};
use super::cache;
use super::limiter;
use super::policy::{Service, ServicePolicy};
use super::types::{Fetched, GetOptions};
use super::wire::{read_body, Outcome, Wire};
use crate::error::{Error, Result};

pub(crate) struct ServiceState {
    pub(crate) policy: ServicePolicy,
    pub(crate) limiter: limiter::Limiter,
    pub(crate) breaker: Mutex<breaker::CircuitBreaker>,
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
    #[allow(unused_assignments, clippy::too_many_arguments)]
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

            let retry_after = match self.send(&parsed, &wire, opts, timeout, cap).await {
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
                }
                Ok(Outcome::ServerError { status, body }) => {
                    last_err = Some(Error::Status {
                        status,
                        url: key.clone(),
                        retry_after: None,
                        body,
                    });
                    self.record_failure(&state);
                    None
                }
                Ok(Outcome::ClientError { status, body }) => {
                    // 4xx is our problem, not the service being down: no retry,
                    // no breaker. (qBittorrent 403 triggers a re-login upstream.)
                    self.record_success(&state);
                    return Err(Error::Status {
                        status,
                        url: key.clone(),
                        retry_after: None,
                        body,
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
                None => backoff::jitter(backoff::delay_for(attempt, &retry.schedule))
                    .min(retry.max_delay),
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
            408 | 425 | 500..=599 => {
                let body = read_body(resp, cap, url.as_str(), timeout).await.ok();
                let body_str = body.and_then(|b| String::from_utf8(b).ok());
                Ok(Outcome::ServerError {
                    status,
                    body: body_str,
                })
            }
            _ => {
                let body = read_body(resp, cap, url.as_str(), timeout).await.ok();
                let body_str = body.and_then(|b| String::from_utf8(b).ok());
                Ok(Outcome::ClientError {
                    status,
                    body: body_str,
                })
            }
        }
    }
}
