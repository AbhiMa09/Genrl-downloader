//! The matching engine: the app resolves every show to a MAL id first, then
//! searches Nyaa with names that belong to that id.
//!
//! Nyaa uploaders use romaji, English, abbreviations or typos, so one title is
//! never enough. The engine:
//!
//! 1. builds the show's **name set** ([`names::NameSet`]) and reads every
//!    *other* show's names as the reject set, so `Kimi no Todoke` can never
//!    pass for `Kimi no Na wa`;
//! 2. searches Nyaa **once per name in priority order** (romaji first),
//!    stopping as soon as a name returns enough good candidates, which keeps
//!    the request count low;
//! 3. scores each result with [`score::evaluate`] against the show;
//! 4. **decides**: best score at or above the threshold *and* far enough ahead
//!    of the runner-up proceeds automatically; otherwise the job pauses with a
//!    Match Review card and the top candidates, why each scored as it did;
//!    a healthy-enough torrent still has to clear the seed-health buckets.
//!
//! The network step is the only async part; scoring and the decision are pure
//! functions, which is what the fixture suite tests.

pub mod decide;
pub mod search;
pub mod types;

pub use decide::*;
pub use search::*;
pub use types::*;

#[cfg(test)]
mod tests;
