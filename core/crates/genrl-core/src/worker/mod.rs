//! The five steps: search, choose, add, monitor, finalize.
//!
//! Each step is one async function on [`Engine`] that (a) reads what it needs,
//! (b) does its work through the [`Gate`], and (c) writes the *next* transition
//! before returning. Because a transition lands before the work it enables, a
//! process killed anywhere inside a step leaves the job in the state that step
//! was about to run — so [`crate::queue::reconcile`] can finish the job by
//! re-running it, and every step is idempotent by construction:
//!
//! * `search` writes `torrents` rows keyed by `info_hash` (re-seeing a release
//!   updates it);
//! * `choose` re-derives its verdict from those rows — no network, so a crash
//!   costs nothing;
//! * `add` re-adding the same magnet is a no-op in qBittorrent, keyed by hash;
//! * `monitor` only reads and timestamps;
//! * `finalize` writes the episode row once and calls the library hook, which
//!   the phase-6 implementation makes idempotent (a size-and-existence check
//!   guards the no-op used here).

mod engine;
mod helpers;
mod inbox;
mod steps_acquire;
mod steps_finish;
mod types;

pub use engine::*;
pub use inbox::*;
pub use types::*;
