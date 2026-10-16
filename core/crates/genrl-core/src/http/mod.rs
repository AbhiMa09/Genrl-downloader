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
pub mod gate;
pub mod limiter;
pub mod policy;
pub mod types;
pub mod validate;
pub(crate) mod wire;

pub use breaker::State as BreakerState;
pub use gate::*;
pub use policy::{RetryPolicy, Service, ServicePolicy, ServicePolicy as Policy};
pub use types::*;
pub use validate::*;

#[cfg(test)]
mod tests {
    use super::policy::RateSpec;
    use super::*;
    use std::time::Duration;

    #[test]
    fn accepts_magnets_and_public_torrent_links() {
        assert!(is_acceptable_link(
            "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=x"
        ));
        assert!(is_acceptable_link("https://nyaa.si/download/123.torrent"));
        assert!(!is_acceptable_link("ftp://example.com/x.torrent"));
        assert!(!is_acceptable_link("file:///etc/passwd"));
        assert!(
            !is_acceptable_link("http://nyaa.si/download/123.torrent"),
            "https only"
        );
    }

    #[test]
    fn refuses_private_and_loopback_targets() {
        assert!(!is_acceptable_link("http://127.0.0.1:8080/x.torrent"));
        assert!(!is_acceptable_link("https://192.168.1.10/x.torrent"));
        assert!(!is_acceptable_link("https://10.0.0.1/x.torrent"));
        assert!(!is_acceptable_link("https://localhost/x.torrent"));
        assert!(!is_acceptable_link(
            "http://169.254.169.254/latest/meta-data"
        ));
        assert!(!is_acceptable_link("not a url"));
    }

    #[test]
    fn rate_specs_match_the_design() {
        let jikan = ServicePolicy::for_service(Service::Jikan);
        assert_eq!(jikan.rate.permits_per_second, 2.0);
        assert_eq!(jikan.rate.per_minute_cap, Some(50));
        assert_eq!(jikan.cache_ttl, Some(Duration::from_secs(7 * 24 * 3600)));

        let nyaa = ServicePolicy::for_service(Service::Nyaa);
        assert_eq!(
            nyaa.rate.permits_per_second, 0.5,
            "one request per 2 seconds"
        );
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
