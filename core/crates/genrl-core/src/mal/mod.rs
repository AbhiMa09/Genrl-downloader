//! The real MAL sync (phase 5): OAuth with PKCE, the list, the diff, and the
//! season chain.
//!
//! The design's limit-proofing table: *"Sync at most every 30 minutes, ETag or
//! last-updated checks, refresh the OAuth token 5 minutes before expiry, store
//! the refresh token in the system keyring"* — and its security note: *"The
//! MAL OAuth redirect uses a local loopback port with PKCE, so no client
//! secret is stored."*
//!
//! Three rules the code follows:
//!
//! * **No client secret anywhere.** PKCE's S256 challenge is generated here;
//!   the only credential stored is the refresh token, in the keyring
//!   ([`crate::secrets`]). The short-lived access token lives in memory for
//!   the process only and is re-minted by a refresh on launch, so no
//!   plaintext credential ever lands in the database, a file or a log line.
//! * **Every request through the gate.** The limiter, cache, backoff and
//!   breaker of [`crate::http`] apply; a MAL outage lands an Activity-log
//!   warning, never a stuck queue.
//! * **A relation failure never misgroups.** A sequel or prequel lookup that
//!   fails leaves that entry its own one-season chain — a missing chain is
//!   safe, a wrong one is worse.
//!
//! The list comes back as the [`crate::planner::ListEntry`]s the planner already
//! works on, with `franchise`/`season`/`episode_offset` filled from MAL's
//! sequel and prequel relations, so [`crate::planner::plan`] runs unchanged on
//! a real list.

mod client;
mod connect;
mod jikan_parse;
mod jikan_sync;
mod login;
mod models;
mod pkce;
mod sync;
mod token;

pub use client::*;
pub use connect::*;
pub use jikan_parse::*;
pub use jikan_sync::*;
pub use login::*;
pub use models::*;
pub use pkce::*;
pub use sync::*;
pub use token::*;
