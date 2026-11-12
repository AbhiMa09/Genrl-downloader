//! Torrent selection: hard filters, the accept score, and ranking.
//!
//! Three stages, in order:
//!
//! 1. **Hard filters** — wrong episode, wrong type, below the resolution
//!    floor, excluded codec, out-of-range size, zero seeders, blocked
//!    uploader: reject outright, no points awarded.
//! 2. **Signal score** (max 105, minus 20 for a wrong year) — the design's
//!    accept table: title match 50, episode match 30, type match 10, batch
//!    when fully aired 15. Compared against `Settings::score_threshold`
//!    (default 80) to decide "download" versus "ask".
//! 3. **Ranking** — how to order the candidates that passed: group priority,
//!    resolution, seeders on a log scale, trusted/re-release, dual audio,
//!    recency. Ranking never rescues a candidate the signal score doubts.

pub mod evaluate;
pub mod helpers;
pub mod size;
pub mod types;

#[cfg(test)]
mod tests;

pub use evaluate::*;
pub use size::*;
pub use types::*;
