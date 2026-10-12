//! Token-bucket rate limiter.
//!
//! Each service gets one bucket (plus an optional per-minute bucket), so no
//! matter how many jobs run at once the app cannot exceed that service's budget.

use std::sync::Mutex;
use std::time::Duration;

// tokio's clock, not std's: under `tokio::test(start_paused = true)` the
// virtual clock is what actually advances, and the limiter must follow it.
use tokio::time::Instant;

use super::policy::RateSpec;

struct Bucket {
    rate: f64,
    capacity: f64,
    tokens: f64,
    last_refill: Instant,
}

impl Bucket {
    fn new(rate: f64, capacity: f64) -> Self {
        Bucket {
            rate,
            capacity,
            tokens: capacity,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_refill);
        if elapsed.is_zero() {
            return;
        }
        self.tokens = (self.tokens + elapsed.as_secs_f64() * self.rate).min(self.capacity);
        self.last_refill = now;
    }

    /// How long until one token is available (zero if one is available now),
    /// and whether a token could be taken right now.
    fn probe(&mut self, now: Instant) -> (Duration, bool) {
        self.refill(now);
        if self.tokens >= 1.0 {
            (Duration::ZERO, true)
        } else {
            let missing = 1.0 - self.tokens;
            let secs = missing / self.rate;
            (Duration::from_secs_f64(secs.max(0.0)), false)
        }
    }

    fn take(&mut self) {
        self.tokens -= 1.0;
    }
}

pub struct Limiter {
    buckets: Mutex<Vec<Bucket>>,
    jitter: bool,
}

impl Limiter {
    pub fn new(spec: &RateSpec) -> Self {
        let mut buckets = vec![Bucket::new(
            spec.permits_per_second,
            spec.burst.max(1) as f64,
        )];
        if let Some(per_minute) = spec.per_minute_cap {
            if per_minute > 0 {
                // 60 tokens a minute, refilled one at a time.
                buckets.push(Bucket::new(per_minute as f64 / 60.0, per_minute as f64));
            }
        }
        Limiter {
            buckets: Mutex::new(buckets),
            jitter: spec.jitter,
        }
    }

    /// Wait until every bucket can spare a token, then spend them.
    pub async fn acquire(&self) {
        loop {
            let wait = {
                let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
                let now = Instant::now();
                let mut longest = Duration::ZERO;
                let mut ready = true;
                for bucket in buckets.iter_mut() {
                    let (wait, ok) = bucket.probe(now);
                    if !ok {
                        ready = false;
                    }
                    longest = longest.max(wait);
                }
                if ready {
                    for bucket in buckets.iter_mut() {
                        bucket.take();
                    }
                    return;
                }
                longest
            };

            let mut sleep_for = if wait.is_zero() {
                // Refill granularity: don't spin.
                Duration::from_millis(1)
            } else {
                wait
            };
            if self.jitter {
                sleep_for = add_jitter(sleep_for);
            }
            tokio::time::sleep(sleep_for).await;
        }
    }

    /// Snapshot of the wait a new caller would face right now (for diagnostics).
    pub fn queued_wait(&self) -> Duration {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        buckets
            .iter_mut()
            .map(|bucket| bucket.probe(now).0)
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

/// ±20% random spread so callers do not arrive in lockstep.
fn add_jitter(d: Duration) -> Duration {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let factor = rng.gen_range(0.8..1.2);
    Duration::from_secs_f64((d.as_secs_f64() * factor).max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn allows_burst_then_spaces_out() {
        let limiter = Limiter::new(&RateSpec {
            permits_per_second: 10.0,
            burst: 2,
            per_minute_cap: None,
            jitter: false,
        });

        // Two permits are free immediately.
        let started = Instant::now();
        limiter.acquire().await;
        limiter.acquire().await;
        assert_eq!(started.elapsed(), Duration::ZERO);

        // The third waits ~100 ms.
        limiter.acquire().await;
        assert!(started.elapsed() >= Duration::from_millis(90), "{:?}", started.elapsed());
    }

    #[tokio::test(start_paused = true)]
    async fn honours_the_per_minute_cap() {
        let limiter = Limiter::new(&RateSpec {
            permits_per_second: 1000.0,
            burst: 1000,
            per_minute_cap: Some(60),
            jitter: false,
        });

        let started = Instant::now();
        for _ in 0..60 {
            limiter.acquire().await;
        }
        // The 61st permit needs the minute bucket to refill (1/s).
        limiter.acquire().await;
        assert!(started.elapsed() >= Duration::from_millis(900), "{:?}", started.elapsed());
    }

    #[tokio::test(start_paused = true)]
    async fn nyaa_gets_one_request_every_two_seconds() {
        let limiter = Limiter::new(&RateSpec {
            permits_per_second: 0.5,
            burst: 1,
            per_minute_cap: None,
            jitter: false,
        });
        let started = Instant::now();
        limiter.acquire().await;
        limiter.acquire().await;
        limiter.acquire().await;
        assert!(started.elapsed() >= Duration::from_secs(4), "{:?}", started.elapsed());
    }
}
