//! The season chain, and the pure decision function that says what a release
//! is good for.
//!
//! The MAL id is the identity of a show, and **each season is its own MAL
//! entry**. What to download follows from the user's list — the status, the
//! watched count and where the release's number sits against the entry's
//! episode count — never from the release name alone.
//!
//! Two pure functions do all of it, which is why the Phase 3 check can run as
//! fixtures with a fake list and no network:
//!
//! * [`SeasonChain::wanted`] — what the list wants next (or why it wants
//!   nothing): Completed entries are walked past, the first entry that still
//!   wants work decides. Fully-watched is skipped with a suggestion, never an
//!   error. On hold, dropped and Plan to watch are skipped unless switched on.
//! * [`decide`] — given that want and one parsed release: download, skip (with
//!   a reason), ask (queue a question) or not found. A season the chain does
//!   not have is *never* substituted with another, and an episode number
//!   beyond the entry's count is treated as absolute and mapped through the
//!   earlier seasons' cumulative counts.

pub mod decide;
pub mod types;
pub mod walk;

pub use decide::*;
pub use types::*;

#[cfg(test)]
mod tests;
