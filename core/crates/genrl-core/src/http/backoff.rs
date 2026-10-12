//! The backoff rule: 5 s, 30 s, 2 min, 10 min, 1 hour, each with random jitter.
//!
//! Used both by the HTTP layer (first steps only) and by the job queue, which
//! owns the full schedule when a call gives up in-request.

use std::time::Duration;

/// Delay before retry number `attempt` (1-based: attempt 1 already failed).
/// Steps past the end of the schedule repeat the last one.
pub fn delay_for(attempt: u32, schedule: &[Duration]) -> Duration {
    if schedule.is_empty() {
        return Duration::ZERO;
    }
    let idx = attempt.max(1) as usize - 1;
    let idx = idx.min(schedule.len() - 1);
    schedule[idx]
}

/// ±10% jitter so retries from many jobs do not stampede together.
pub fn jitter(delay: Duration) -> Duration {
    use rand::Rng;
    if delay.is_zero() {
        return delay;
    }
    let mut rng = rand::thread_rng();
    let factor = rng.gen_range(0.9..1.1);
    Duration::from_secs_f64(delay.as_secs_f64() * factor)
}

/// Parse `Retry-After` when it is delta-seconds (the form we act on).
/// HTTP-date values fall back to the regular backoff step.
pub fn parse_retry_after(header: Option<&str>) -> Option<Duration> {
    let raw = header?.trim();
    let secs: u64 = raw.parse().ok()?;
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_the_design_schedule() {
        let schedule = super::super::policy::default_backoff_schedule();
        assert_eq!(delay_for(1, &schedule), Duration::from_secs(5));
        assert_eq!(delay_for(2, &schedule), Duration::from_secs(30));
        assert_eq!(delay_for(3, &schedule), Duration::from_secs(120));
        assert_eq!(delay_for(4, &schedule), Duration::from_secs(600));
        assert_eq!(delay_for(5, &schedule), Duration::from_secs(3600));
        assert_eq!(delay_for(6, &schedule), Duration::from_secs(3600), "repeats the last step");
    }

    #[test]
    fn jitter_stays_near_the_nominal_delay() {
        let d = Duration::from_secs(10);
        for _ in 0..200 {
            let j = jitter(d);
            assert!(j >= Duration::from_secs(9) && j <= Duration::from_secs(11), "{j:?}");
        }
    }

    #[test]
    fn parses_retry_after_seconds() {
        assert_eq!(parse_retry_after(Some("42")), Some(Duration::from_secs(42)));
        assert_eq!(parse_retry_after(Some(" 7 ")), Some(Duration::from_secs(7)));
        assert_eq!(parse_retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(parse_retry_after(None), None);
    }
}
