//! Circuit breaker.
//!
//! After five consecutive failures a service is marked open for 15 minutes.
//! Requests fail fast with [`crate::Error::BreakerOpen`] and the job queue parks
//! the work in `blocked` instead of hammering a service that is already down.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Closed,
    Open,
    /// Cooldown elapsed: one trial request is allowed through.
    HalfOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Reject { retry_in: Duration },
}

pub struct CircuitBreaker {
    threshold: u32,
    cooldown: Duration,
    failures: u32,
    state: State,
    opened_at: Option<Instant>,
    probe_in_flight: bool,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, cooldown: Duration) -> Self {
        CircuitBreaker {
            threshold: threshold.max(1),
            cooldown,
            failures: 0,
            state: State::Closed,
            opened_at: None,
            probe_in_flight: false,
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    /// May a request go out right now?
    pub fn check(&mut self, now: Instant) -> Verdict {
        match self.state {
            State::Closed => Verdict::Allow,
            State::Open => match self.opened_at {
                Some(at) if now.duration_since(at) >= self.cooldown => {
                    self.state = State::HalfOpen;
                    self.probe_in_flight = true;
                    Verdict::Allow
                }
                Some(at) => Verdict::Reject {
                    retry_in: self.cooldown.saturating_sub(now.duration_since(at)),
                },
                None => Verdict::Allow,
            },
            State::HalfOpen => {
                if self.probe_in_flight {
                    // Another trial is already running; do not pile on.
                    Verdict::Reject {
                        retry_in: Duration::from_millis(250),
                    }
                } else {
                    self.probe_in_flight = true;
                    Verdict::Allow
                }
            }
        }
    }

    /// Would [`check`] allow a request right now — without consuming the
    /// half-open probe?
    ///
    /// The queue asks this before resuming a blocked job: the cooldown may
    /// have elapsed (the breaker moves to half-open here, so the *next* request
    /// claims the probe), but nothing may be spent on the question itself.
    pub fn peek(&mut self, now: Instant) -> bool {
        match self.state {
            State::Closed => true,
            State::Open => match self.opened_at {
                Some(at) if now.duration_since(at) >= self.cooldown => {
                    self.state = State::HalfOpen;
                    true
                }
                _ => false,
            },
            State::HalfOpen => !self.probe_in_flight,
        }
    }

    pub fn on_success(&mut self) {
        self.failures = 0;
        self.state = State::Closed;
        self.opened_at = None;
        self.probe_in_flight = false;
    }

    pub fn on_failure(&mut self, now: Instant) {
        self.probe_in_flight = false;
        match self.state {
            State::HalfOpen => {
                // The trial failed: straight back to open.
                self.state = State::Open;
                self.opened_at = Some(now);
                self.failures = self.threshold;
            }
            _ => {
                self.failures = self.failures.saturating_add(1);
                if self.failures >= self.threshold {
                    self.state = State::Open;
                    self.opened_at = Some(now);
                }
            }
        }
    }

    /// Consecutive failures recorded so far.
    pub fn failures(&self) -> u32 {
        self.failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaker() -> CircuitBreaker {
        CircuitBreaker::new(5, Duration::from_secs(900))
    }

    #[test]
    fn opens_after_five_consecutive_failures() {
        let mut b = breaker();
        let now = Instant::now();
        for i in 0..4 {
            b.on_failure(now);
            assert_eq!(b.state(), State::Closed, "still closed at failure {i}");
        }
        b.on_failure(now);
        assert_eq!(b.state(), State::Open);

        match b.check(now) {
            Verdict::Reject { retry_in } => assert!(retry_in > Duration::from_secs(800)),
            Verdict::Allow => panic!("open breaker must reject"),
        }
    }

    #[test]
    fn success_resets_the_count() {
        let mut b = breaker();
        let now = Instant::now();
        for _ in 0..4 {
            b.on_failure(now);
        }
        b.on_success();
        assert_eq!(b.failures(), 0);
        assert_eq!(b.state(), State::Closed);
        for _ in 0..4 {
            b.on_failure(now);
        }
        assert_eq!(b.state(), State::Closed, "fifth failure needed again");
    }

    #[test]
    fn half_open_allows_one_probe_then_rejects() {
        let mut b = breaker();
        let t0 = Instant::now();
        for _ in 0..5 {
            b.on_failure(t0);
        }
        let later = t0 + Duration::from_secs(901);
        assert_eq!(b.check(later), Verdict::Allow, "cooldown elapsed");
        assert_eq!(b.state(), State::HalfOpen);
        assert!(matches!(b.check(later), Verdict::Reject { .. }), "no pile-up");
    }

    #[test]
    fn failed_probe_reopens_successful_probe_closes() {
        let mut b = breaker();
        let t0 = Instant::now();
        for _ in 0..5 {
            b.on_failure(t0);
        }
        let later = t0 + Duration::from_secs(901);
        b.check(later);
        b.on_failure(later);
        assert_eq!(b.state(), State::Open);

        let even_later = later + Duration::from_secs(901);
        b.check(even_later);
        b.on_success();
        assert_eq!(b.state(), State::Closed);
    }
}
