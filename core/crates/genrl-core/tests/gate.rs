//! Phase 1 check from the roadmap: "mock-server tests for 429 and timeout pass."
//!
//! Every test drives the real [`Gate`] — limiter, cache, backoff, breaker —
//! against a local mock server, never the real services.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use genrl_core::http::{
    breaker, policy::RateSpec, GetOptions, RetryPolicy, Service, ServicePolicy,
};
use genrl_core::{Db, Gate};
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

/// A fast, otherwise identical policy so tests do not wait on production
/// backoff steps: same breaker thresholds and caps, millisecond schedule.
fn fast_policy() -> ServicePolicy {
    ServicePolicy {
        rate: RateSpec {
            permits_per_second: 1000.0,
            burst: 100,
            per_minute_cap: None,
            jitter: false,
        },
        cache_ttl: Some(Duration::from_secs(60)),
        max_body_bytes: 1024 * 1024,
        timeout: Duration::from_millis(500),
        retry: RetryPolicy {
            max_attempts: 3,
            schedule: vec![Duration::from_millis(10), Duration::from_millis(20)],
            max_total_wait: Duration::from_secs(5),
            max_delay: Duration::from_secs(10),
        },
        breaker_threshold: 5,
        breaker_cooldown: Duration::from_secs(900),
    }
}

fn gate_with(policy: ServicePolicy) -> Gate {
    let db = Db::open_in_memory().expect("in-memory database");
    Gate::with_policies(db, &[(Service::Jikan, policy)]).expect("gate")
}

fn fast_gate() -> Gate {
    gate_with(fast_policy())
}

/// Always the same response, counting how many requests actually arrived.
struct Fixed {
    hits: Arc<AtomicUsize>,
    template: ResponseTemplate,
}

impl Fixed {
    fn new(template: ResponseTemplate) -> (Arc<AtomicUsize>, Fixed) {
        let hits = Arc::new(AtomicUsize::new(0));
        (
            hits.clone(),
            Fixed {
                hits,
                template,
            },
        )
    }
}

impl Respond for Fixed {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.hits.fetch_add(1, Ordering::SeqCst);
        self.template.clone()
    }
}

/// `first` for the first `fail_times` requests, then `then` forever.
struct Sequence {
    hits: Arc<AtomicUsize>,
    fail_times: usize,
    first: ResponseTemplate,
    then: ResponseTemplate,
}

impl Sequence {
    fn new(
        fail_times: usize,
        first: ResponseTemplate,
        then: ResponseTemplate,
    ) -> (Arc<AtomicUsize>, Sequence) {
        let hits = Arc::new(AtomicUsize::new(0));
        (
            hits.clone(),
            Sequence {
                hits,
                fail_times,
                first,
                then,
            },
        )
    }
}

impl Respond for Sequence {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        let n = self.hits.fetch_add(1, Ordering::SeqCst);
        if n < self.fail_times {
            self.first.clone()
        } else {
            self.then.clone()
        }
    }
}

// ---------------------------------------------------------------------------
// 429 handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retries_a_429_and_honors_retry_after() {
    let server = MockServer::start().await;
    let (hits, responder) = Sequence::new(
        1,
        ResponseTemplate::new(429).insert_header("Retry-After", "1"),
        ResponseTemplate::new(200).set_body_string("feed"),
    );
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let gate = fast_gate();
    let started = Instant::now();
    let fetched = gate
        .get(Service::Jikan, &format!("{}/feed", server.uri()))
        .await
        .expect("second attempt succeeds");

    assert_eq!(fetched.status, 200);
    assert_eq!(fetched.text().unwrap(), "feed");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "one 429 then one success");

    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_secs(1),
        "Retry-After: 1 was ignored (waited {elapsed:?})"
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "should not have fallen back to a longer backoff step ({elapsed:?})"
    );
}

#[tokio::test]
async fn persistent_429_raises_rate_limited_then_opens_the_breaker() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(429).insert_header("Retry-After", "0"));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.retry.max_attempts = 2;
    policy.retry.max_total_wait = Duration::from_millis(50);
    let gate = gate_with(policy);
    let url = format!("{}/limited", server.uri());

    let err = gate.get(Service::Jikan, &url).await.unwrap_err();
    assert!(
        matches!(err, genrl_core::Error::RateLimited { .. }),
        "got {err:?}"
    );

    // Keep going: five consecutive failures trip the breaker.
    for _ in 0..10 {
        if gate.breaker_state(Service::Jikan) == breaker::State::Open {
            break;
        }
        let _ = gate.get(Service::Jikan, &url).await;
    }
    assert_eq!(
        gate.breaker_state(Service::Jikan),
        breaker::State::Open,
        "five consecutive 429s must open the breaker"
    );

    let before = hits.load(Ordering::SeqCst);
    let err = gate.get(Service::Jikan, &url).await.unwrap_err();
    assert!(
        matches!(err, genrl_core::Error::BreakerOpen { .. }),
        "expected BreakerOpen, got {err:?}"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        before,
        "an open breaker must not touch the network"
    );
}

// ---------------------------------------------------------------------------
// Timeout handling
// ---------------------------------------------------------------------------

#[tokio::test]
async fn times_out_when_the_server_is_too_slow_and_retries() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(
        ResponseTemplate::new(200)
            .set_body_string("never arrives in time")
            .set_delay(Duration::from_secs(30)),
    );
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.timeout = Duration::from_millis(150);
    policy.retry.max_attempts = 2;
    policy.retry.max_total_wait = Duration::from_millis(500);
    let gate = gate_with(policy);

    let started = Instant::now();
    let err = gate
        .get(Service::Jikan, &format!("{}/slow", server.uri()))
        .await
        .unwrap_err();
    let elapsed = started.elapsed();

    assert!(err.is_timeout(), "expected a timeout, got {err:?}");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "timeout is retried");
    assert!(
        elapsed < Duration::from_secs(3),
        "must give up quickly, waited {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// Retry, breaker and cache behaviour
// ---------------------------------------------------------------------------

#[tokio::test]
async fn retries_server_errors_then_succeeds() {
    let server = MockServer::start().await;
    let (_hits, responder) = Sequence::new(
        1,
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("recovered"),
    );
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let gate = fast_gate();
    let fetched = gate
        .get(Service::Jikan, &format!("{}/flaky", server.uri()))
        .await
        .expect("recovers");
    assert_eq!(fetched.text().unwrap(), "recovered");
}

#[tokio::test]
async fn five_consecutive_failures_open_the_breaker() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(503));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.retry.max_attempts = 2;
    policy.retry.max_total_wait = Duration::from_millis(50);
    let gate = gate_with(policy);
    let url = format!("{}/dead", server.uri());

    for _ in 0..5 {
        if gate.breaker_state(Service::Jikan) == breaker::State::Open {
            break;
        }
        let _ = gate.get(Service::Jikan, &url).await;
    }

    assert_eq!(
        gate.breaker_state(Service::Jikan),
        breaker::State::Open,
        "breaker must open after five consecutive failures"
    );
    assert_eq!(
        hits.load(Ordering::SeqCst),
        5,
        "exactly five failed attempts before the breaker opened"
    );

    // It stays open: the next call fails fast without a request.
    let err = gate.get(Service::Jikan, &url).await.unwrap_err();
    assert!(matches!(err, genrl_core::Error::BreakerOpen { .. }));
    assert_eq!(hits.load(Ordering::SeqCst), 5, "no request while open");
    assert_eq!(
        gate.policy(Service::Jikan).breaker_cooldown,
        Duration::from_secs(900),
        "cooldown is 15 minutes"
    );
}

#[tokio::test]
async fn breaker_closes_after_a_successful_probe() {
    let server = MockServer::start().await;
    let (_hits, responder) = Sequence::new(
        5,
        ResponseTemplate::new(503),
        ResponseTemplate::new(200).set_body_string("back up"),
    );
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.retry.max_attempts = 1;
    policy.breaker_cooldown = Duration::from_millis(100);
    let gate = gate_with(policy);
    let url = format!("{}/recovering", server.uri());

    for _ in 0..5 {
        let _ = gate.get(Service::Jikan, &url).await;
    }
    assert_eq!(gate.breaker_state(Service::Jikan), breaker::State::Open);

    // Cooldown elapses, one probe goes out and succeeds.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let fetched = gate.get(Service::Jikan, &url).await.expect("probe");
    assert_eq!(fetched.text().unwrap(), "back up");
    assert_eq!(
        gate.breaker_state(Service::Jikan),
        breaker::State::Closed,
        "successful probe closes the breaker"
    );
}

#[tokio::test]
async fn serves_fresh_responses_from_cache() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(200).set_body_string("cached-body"));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let gate = fast_gate();
    let url = format!("{}/anime/1", server.uri());

    let first = gate.get(Service::Jikan, &url).await.unwrap();
    assert!(!first.from_cache);
    assert_eq!(first.text().unwrap(), "cached-body");

    let second = gate.get(Service::Jikan, &url).await.unwrap();
    assert!(second.from_cache, "second call must be served from cache");
    assert_eq!(second.text().unwrap(), "cached-body");
    assert_eq!(hits.load(Ordering::SeqCst), 1, "one request reached the server");
}

#[tokio::test]
async fn stale_entries_revalidate_with_if_none_match() {
    let server = MockServer::start().await;

    let conditional_hits = Arc::new(AtomicUsize::new(0));
    let counter = conditional_hits.clone();
    struct EtagServer {
        conditional_hits: Arc<AtomicUsize>,
    }
    impl Respond for EtagServer {
        fn respond(&self, request: &Request) -> ResponseTemplate {
            let conditional = request
                .headers
                .get("if-none-match")
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("\"v1\""))
                .unwrap_or(false);
            if conditional {
                self.conditional_hits.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(304)
            } else {
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"v1\"")
                    .set_body_string("fresh copy")
            }
        }
    }
    Mock::given(method("GET"))
        .respond_with(EtagServer {
            conditional_hits: counter,
        })
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.cache_ttl = Some(Duration::ZERO); // always stale
    let gate = gate_with(policy);
    let url = format!("{}/me/list", server.uri());

    let first = gate.get(Service::Jikan, &url).await.unwrap();
    assert_eq!(first.text().unwrap(), "fresh copy");
    assert_eq!(first.etag.as_deref(), Some("\"v1\""));

    let second = gate.get(Service::Jikan, &url).await.unwrap();
    assert_eq!(second.status, 304);
    assert!(second.from_cache, "304 serves the copy we already have");
    assert_eq!(second.text().unwrap(), "fresh copy", "body preserved");
    assert_eq!(
        conditional_hits.load(Ordering::SeqCst),
        1,
        "revalidation must send If-None-Match"
    );
}

// ---------------------------------------------------------------------------
// Hard filters
// ---------------------------------------------------------------------------

#[tokio::test]
async fn does_not_retry_client_errors() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(404));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let gate = fast_gate();
    let err = gate
        .get(Service::Jikan, &format!("{}/missing", server.uri()))
        .await
        .unwrap_err();

    assert!(
        matches!(err, genrl_core::Error::Status { status: 404, .. }),
        "got {err:?}"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1, "4xx must not be retried");
    assert_eq!(
        gate.breaker_state(Service::Jikan),
        breaker::State::Closed,
        "4xx is not the service being down"
    );
}

#[tokio::test]
async fn refuses_oversized_bodies() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(200).set_body_raw(
        vec![b'x'; 64 * 1024],
        "text/plain",
    ));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let mut policy = fast_policy();
    policy.max_body_bytes = 1024;
    let gate = gate_with(policy);

    let err = gate
        .get(Service::Jikan, &format!("{}/huge", server.uri()))
        .await
        .unwrap_err();
    assert!(
        matches!(err, genrl_core::Error::BodyTooLarge { cap: 1024, .. }),
        "got {err:?}"
    );
    assert_eq!(hits.load(Ordering::SeqCst), 1, "an oversized body is not retried");
}

#[tokio::test]
async fn skips_the_cache_on_request() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(200).set_body_string("v"));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let gate = fast_gate();
    let url = format!("{}/search?q=x", server.uri());

    gate.get(Service::Jikan, &url).await.unwrap();
    let opts = GetOptions {
        skip_cache: true,
        ..Default::default()
    };
    gate.get_with(Service::Jikan, &url, &opts).await.unwrap();

    assert_eq!(hits.load(Ordering::SeqCst), 2, "skip_cache must reach the server");
    // …and the refresh replaces the cached copy.
    let third = gate.get(Service::Jikan, &url).await.unwrap();
    assert!(third.from_cache);
    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

// ---------------------------------------------------------------------------
// Rate limiting (the budget that keeps us off Nyaa's ban list)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn nyaa_budget_is_one_request_per_two_seconds() {
    let server = MockServer::start().await;
    let (hits, responder) = Fixed::new(ResponseTemplate::new(200).set_body_string("rss"));
    Mock::given(method("GET"))
        .respond_with(responder)
        .mount(&server)
        .await;

    let db = Db::open_in_memory().unwrap();
    let gate =
        Gate::with_policies(db, &[(Service::Nyaa, ServicePolicy::for_service(Service::Nyaa))])
            .unwrap();

    // The first request fits in the burst allowance…
    let url = format!("{}/rss?q=0", server.uri());
    gate.get(Service::Nyaa, &url).await.unwrap();
    assert_eq!(hits.load(Ordering::SeqCst), 1);

    // …and now the single-token bucket is empty, so the next caller must wait
    // the full two seconds. (The exact waiting is covered by the limiter's own
    // tests; here we prove the gate wires the Nyaa budget in.)
    let wait = gate.rate_wait(Service::Nyaa);
    assert!(
        wait >= Duration::from_millis(1500) && wait <= Duration::from_secs(3),
        "Nyaa budget should be one request per 2s, next call waits {wait:?}"
    );
}

// ---------------------------------------------------------------------------

/// Sanity: default policies are in place and every breaker starts closed.
#[test]
fn default_policies_are_in_place() {
    let db = Db::open_in_memory().unwrap();
    let gate = Gate::new(db).unwrap();
    for service in Service::ALL {
        assert_eq!(gate.breaker_state(service), breaker::State::Closed);
    }
    assert_eq!(
        gate.policy(Service::Nyaa).rate.permits_per_second,
        0.5,
        "Nyaa: one request per 2 seconds"
    );
}
