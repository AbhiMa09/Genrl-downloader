//! The library step: file placement, metadata, and folder decoration (Phase 6).
//!
//! When a torrent finishes, the finalize step files it in a predictable layout:
//!
//! ```text
//! Anime/
//!   Kimi no Na wa (2016)/
//!     synopsis.txt
//!     folder.png
//!     .directory
//!     Kimi no Na wa (2016).mkv
//!   Show Title (2024)/
//!     synopsis.txt
//!     folder.png
//!     .directory
//!     Show Title - S01E01.mkv
//!     Show Title - S01E02.mkv
//! ```
//!
//! The step follows the six design rules:
//!
//! 1. **Verify the file:** video extension allowlist, file existence and size.
//! 2. **Resolve show folder:** matched by `.mal_id` marker file first so
//!    renaming folders never loses show identity; sanitized names truncate at
//!    120 characters and strip control/bidi characters.
//! 3. **Write synopsis.txt:** rich metadata from Jikan, sanitized against
//!    injection; writes a placeholder if Jikan is offline/down instead of
//!    failing the download.
//! 4. **Download poster & set folder icon:** square `folder.png` via `magick`
//!    or pure-Rust fallback; sets `.directory` (KDE) or `gio` (GNOME).
//! 5. **Move or hardlink:** according to settings, with cross-device copy fallback
//!    and `chmod 644`.
//! 6. **Handle collisions:** asks in the Inbox (`FileExists`) rather than
//!    overwriting silently, honoring `Keep`, `Replace`, or `KeepBoth`.
//! 7. **Rescan folders:** adopts existing directories and episodes without duplication.

mod finalizer;
mod folders;
mod formatting;
mod icons;
mod rescan;
mod service;
mod types;

pub use folders::*;
pub use formatting::*;
pub use icons::*;
pub use rescan::*;
pub use service::*;
pub use types::*;
