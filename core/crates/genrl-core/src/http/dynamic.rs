use std::sync::Arc;
use std::time::{Duration, Instant};

use super::breaker;
use super::gate::{Gate, ServiceState};
use super::limiter;
use super::policy::ServicePolicy;
use super::types::{Fetched, GetOptions};
use super::wire::Wire;
use crate::error::{Error, Result};

impl Gate {
    /// Retrieve or initialize a dynamic breaker state by key (e.g., "tmdb", "torznab:<id>")
    pub(crate) fn dynamic_state(&self, key: &str) -> Arc<ServiceState> {
        let mut map = self
            .dynamic_states
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(state) = map.get(key) {
            return state.clone();
        }
        let rate = if key.starts_with("tmdb") {
            super::policy::RateSpec {
                permits_per_second: 4.0,
                burst: 10,
                per_minute_cap: None,
                jitter: false,
            }
        } else {
            super::policy::RateSpec {
                permits_per_second: 2.0,
                burst: 5,
                per_minute_cap: None,
                jitter: false,
            }
        };
        let policy = ServicePolicy {
            rate,
            cache_ttl: None,
            timeout: Duration::from_secs(15),
            retry: super::policy::RetryPolicy::default(),
            breaker_threshold: 5,
            breaker_cooldown: Duration::from_secs(900),
            max_body_bytes: 10 * 1024 * 1024,
        };
        let limiter = limiter::Limiter::new(&policy.rate);
        let breaker =
            breaker::CircuitBreaker::new(policy.breaker_threshold, policy.breaker_cooldown);
        let state = Arc::new(ServiceState {
            policy,
            limiter,
            breaker: std::sync::Mutex::new(breaker),
        });
        map.insert(key.to_string(), state.clone());
        state
    }

    /// Check if dynamic breaker allows request without consuming probe
    pub fn would_allow_key(&self, key: &str) -> bool {
        let state = self.dynamic_state(key);
        let mut guard = state.breaker.lock().unwrap_or_else(|e| e.into_inner());
        guard.peek(Instant::now())
    }

    /// GET a URL using a dynamic breaker key through the gate.
    pub async fn get_keyed(&self, breaker_key: &str, url: &str) -> Result<Fetched> {
        self.get_keyed_with(breaker_key, url, &GetOptions::default())
            .await
    }

    /// GET a URL with options using a dynamic breaker key through the gate.
    pub async fn get_keyed_with(
        &self,
        breaker_key: &str,
        url: &str,
        opts: &GetOptions,
    ) -> Result<Fetched> {
        let parsed = reqwest::Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_string()))?;
        let key = parsed.to_string();
        let state = self.dynamic_state(breaker_key);
        self.allow_str(breaker_key, &state)?;
        let wire = Wire::Get {
            if_none_match: None,
        };
        self.dispatch(breaker_key, parsed, key, state, opts, wire, opts.ttl)
            .await
    }
}
