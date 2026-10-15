//! The scheduler, and the launch-time reconcile that survives being killed.
//!
//! The scheduler is deliberately thin: it picks due jobs, respects the pause
//! switch and the per-kind concurrency limits, and dispatches them to
//! [`worker::Engine::step`]. Everything interesting — state, retries, blocking —
//! lives in the steps and the transition table, so the scheduler can be
//! replaced or run headless without changing what a job means.
//!
//! [`reconcile`] is the phase's check. On launch it walks every non-terminal
//! job, compares it with qBittorrent by info hash, and resumes it:
//!
//! | what it finds | what it does |
//! |---|---|
//! | torrent present and complete | finalize |
//! | torrent present and downloading | monitor |
//! | absent after the add step | re-add by magnet (idempotent by hash) |
//! | absent before the add step | redo the step |
//!
//! Because a transition is written *before* the work it enables, every
//! possible kill point leaves the job in exactly one of these cases.

pub mod reconcile;
pub mod scheduler;
pub mod status;

pub use reconcile::*;
pub use scheduler::*;
pub use status::*;
