use std::time::Duration;

/// The design's backoff rule: 5 s, 30 s, 2 min, 10 min, 1 hour, then hold.
pub fn backoff_delay(attempts: i64) -> Duration {
    const LADDER: &[u64] = &[5, 30, 120, 600, 3600];
    let index = (attempts.max(1) - 1) as usize;
    let secs = LADDER.get(index).copied().unwrap_or(3600);
    Duration::from_secs(secs)
}

/// Spread a delay out by up to a quarter, so a whole queue waking at once
/// does not hammer a service that just came back.
///
/// `sample` is a random draw in `0.0 .. 1.0` — the caller supplies the
/// randomness; tests pass `0.0` for an exact ladder.
pub fn with_jitter(delay: Duration, sample: f64) -> Duration {
    let sample = sample.clamp(0.0, 1.0);
    delay + delay.mul_f64(0.25 * sample)
}

/// Start of the UTC day containing `at`.
pub fn start_of_day(at: i64) -> i64 {
    at - at.rem_euclid(86_400)
}
