//! Per-service policies: how often we may call out, how long responses live,
//! how long a request may take, and how hard we retry.
//!
//! Every number here has a counterpart in Settings so it can be overridden.

use std::time::Duration;

/// The five services the app is allowed to talk to. No module talks to the
/// network directly; it asks the [`crate::http::Gate`] for one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    Mal,
    Jikan,
    Nyaa,
    Qbit,
    Images,
}

impl Service {
    pub fn as_str(self) -> &'static str {
        match self {
            Service::Mal => "mal",
            Service::Jikan => "jikan",
            Service::Nyaa => "nyaa",
            Service::Qbit => "qbit",
            Service::Images => "images",
        }
    }

    pub fn parse(s: &str) -> Option<Service> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mal" => Some(Service::Mal),
            "jikan" => Some(Service::Jikan),
            "nyaa" | "nyaa-rss" => Some(Service::Nyaa),
            "qbit" | "qbittorrent" => Some(Service::Qbit),
            "images" | "image" | "posters" => Some(Service::Images),
            _ => None,
        }
    }

    /// Every service, for iteration in tests and diagnostics.
    pub const ALL: [Service; 5] = [
        Service::Mal,
        Service::Jikan,
        Service::Nyaa,
        Service::Qbit,
        Service::Images,
    ];
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Token-bucket rate limit. The design gives each service its own budget:
/// Jikan 2/s and 60/min, Nyaa one per 2 s globally, MAL and qBittorrent gentler.
#[derive(Debug, Clone)]
pub struct RateSpec {
    pub permits_per_second: f64,
    /// Bucket size: how many calls may burst through at once.
    pub burst: u32,
    /// A second, slower bucket for per-minute quotas (Jikan's 60/min).
    pub per_minute_cap: Option<u32>,
    /// Randomise spacing so Nyaa sees irregular traffic.
    pub jitter: bool,
}

/// The design's backoff rule: 5 s, 30 s, 2 min, 10 min, 1 hour, each with jitter.
pub fn default_backoff_schedule() -> Vec<Duration> {
    vec![
        Duration::from_secs(5),
        Duration::from_secs(30),
        Duration::from_secs(120),
        Duration::from_secs(600),
        Duration::from_secs(3600),
    ]
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Give up after this many attempts inside one call.
    pub max_attempts: u32,
    /// Delay before attempt N+1 (extra attempts reuse the last step).
    pub schedule: Vec<Duration>,
    /// A single call never sleeps longer than this in total; the job queue
    /// picks up the remaining backoff steps instead of blocking a worker.
    pub max_total_wait: Duration,
    /// Cap any single sleep, including a very large `Retry-After`.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: 5,
            schedule: default_backoff_schedule(),
            max_total_wait: Duration::from_secs(60),
            max_delay: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServicePolicy {
    pub rate: RateSpec,
    /// Freshness of a cached response. `None` means never cache.
    pub cache_ttl: Option<Duration>,
    /// Response size cap — a hostile RSS feed or a broken host cannot eat RAM.
    pub max_body_bytes: usize,
    /// Per-request timeout (connect + send + body).
    pub timeout: Duration,
    pub retry: RetryPolicy,
    /// After this many consecutive failures the circuit opens and jobs that
    /// need this service wait in `blocked` instead of failing.
    pub breaker_threshold: u32,
    pub breaker_cooldown: Duration,
}

impl ServicePolicy {
    /// Defaults straight from the design's limit-proofing table.
    pub fn for_service(service: Service) -> Self {
        match service {
            // Sync at most every 30 minutes (job level); the limiter keeps
            // individual calls polite while the token lives in the keyring.
            Service::Mal => ServicePolicy {
                rate: RateSpec {
                    permits_per_second: 1.0,
                    burst: 3,
                    per_minute_cap: None,
                    jitter: false,
                },
                cache_ttl: Some(Duration::from_secs(1800)),
                max_body_bytes: 5 * 1024 * 1024,
                timeout: Duration::from_secs(20),
                retry: RetryPolicy::default(),
                breaker_threshold: 5,
                breaker_cooldown: Duration::from_secs(900),
            },
            // Jikan policy: 2 req/s and 50/min cap on top. Cache every show 7 days.
            Service::Jikan => ServicePolicy {
                rate: RateSpec {
                    permits_per_second: 2.0,
                    burst: 2,
                    per_minute_cap: Some(50),
                    jitter: false,
                },
                cache_ttl: Some(Duration::from_secs(7 * 24 * 3600)),
                max_body_bytes: 5 * 1024 * 1024,
                timeout: Duration::from_secs(20),
                retry: RetryPolicy::default(),
                breaker_threshold: 5,
                breaker_cooldown: Duration::from_secs(900),
            },
            // One request per 2 seconds globally, cache each query 10 minutes,
            // jitter, never scrape HTML pages.
            Service::Nyaa => ServicePolicy {
                rate: RateSpec {
                    permits_per_second: 0.5,
                    burst: 1,
                    per_minute_cap: None,
                    jitter: true,
                },
                cache_ttl: Some(Duration::from_secs(600)),
                max_body_bytes: 5 * 1024 * 1024,
                timeout: Duration::from_secs(30),
                retry: RetryPolicy::default(),
                breaker_threshold: 5,
                breaker_cooldown: Duration::from_secs(900),
            },
            // Health check on launch, auto re-login on a 403 (client layer),
            // no caching: every call is a command.
            Service::Qbit => ServicePolicy {
                rate: RateSpec {
                    permits_per_second: 5.0,
                    burst: 10,
                    per_minute_cap: None,
                    jitter: false,
                },
                cache_ttl: None,
                max_body_bytes: 10 * 1024 * 1024,
                timeout: Duration::from_secs(10),
                retry: RetryPolicy {
                    max_attempts: 3,
                    max_total_wait: Duration::from_secs(35),
                    ..RetryPolicy::default()
                },
                breaker_threshold: 5,
                breaker_cooldown: Duration::from_secs(900),
            },
            // Posters: download once, store locally, placeholder on failure.
            Service::Images => ServicePolicy {
                rate: RateSpec {
                    permits_per_second: 2.0,
                    burst: 4,
                    per_minute_cap: None,
                    jitter: false,
                },
                cache_ttl: Some(Duration::from_secs(30 * 24 * 3600)),
                max_body_bytes: 20 * 1024 * 1024,
                timeout: Duration::from_secs(30),
                retry: RetryPolicy {
                    max_attempts: 2,
                    max_total_wait: Duration::from_secs(10),
                    ..RetryPolicy::default()
                },
                breaker_threshold: 5,
                breaker_cooldown: Duration::from_secs(900),
            },
        }
    }
}
