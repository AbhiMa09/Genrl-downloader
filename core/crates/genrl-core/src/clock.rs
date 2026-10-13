//! One clock, injectable.
//!
//! Everything time-shaped — when a job is due, when a torrent counts as
//! stalled, when a question expires — reads the same [`Clock`]. Production
//! uses [`SystemClock`]; tests use [`FakeClock`] and move time by hand, so
//! nothing in the queue suite ever sleeps.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch.
pub trait Clock: Send + Sync {
    fn now(&self) -> i64;
}

/// The wall clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

/// A clock tests move by hand.
#[derive(Debug, Clone, Default)]
pub struct FakeClock {
    at: Arc<AtomicI64>,
}

impl FakeClock {
    pub fn new(at: i64) -> Self {
        FakeClock {
            at: Arc::new(AtomicI64::new(at)),
        }
    }

    /// Move the clock forward (or back) by `seconds`.
    pub fn advance(&self, seconds: i64) {
        self.at.fetch_add(seconds, Ordering::SeqCst);
    }

    /// Set the clock absolutely.
    pub fn set(&self, at: i64) {
        self.at.store(at, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> i64 {
        self.at.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fake_clock_moves_only_when_told() {
        let clock = FakeClock::new(1_000);
        assert_eq!(clock.now(), 1_000);
        clock.advance(60);
        clock.advance(40);
        assert_eq!(clock.now(), 1_100);
        clock.set(2_000);
        assert_eq!(clock.now(), 2_000);

        let shared = clock.clone();
        std::thread::spawn(move || shared.advance(5)).join().unwrap();
        assert_eq!(clock.now(), 2_005, "handles are shared, not copied values");
    }

    #[test]
    fn the_system_clock_is_sane() {
        let now = SystemClock.now();
        // After 2020 and before 2100: catches a broken UNIX_EPOCH conversion.
        assert!(now > 1_577_836_800, "{now}");
        assert!(now < 4_102_444_800, "{now}");
    }
}
