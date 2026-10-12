//! Genrl Downloader core library.
//!
//! Foundation for the whole app (see the system design document):
//!
//! * [`db`] — one SQLite database in WAL mode with versioned migrations.
//! * [`settings`] — every default number in the design lives here, stored in SQLite,
//!   exportable to JSON.
//! * [`rules`] — download preferences (group priorities, resolution, seed thresholds,
//!   blocked uploaders), global and per show.
//! * [`events`] — the activity log shown on the Activity page.
//! * [`logging`] — optional debug logging that rotates by size.
//! * [`http`] — the single shared client layer every external call goes through:
//!   token-bucket limiter, response cache, retry with backoff, circuit breaker.
//! * [`nyaa`], [`filename`], [`score`] — RSS search, release-name parsing,
//!   torrent selection (phase 2).
//! * [`qbit`] — the qBittorrent WebUI client (phase 2).
//! * [`secrets`] — credentials, in the system keyring only.
//!
//! No other module talks to the network directly.

pub mod db;
pub mod error;
pub mod events;
pub mod filename;
pub mod http;
pub mod logging;
pub mod models;
pub mod nyaa;
pub mod qbit;
pub mod rules;
pub mod score;
pub mod secrets;
pub mod settings;

pub use db::Db;
pub use error::{Error, Result};
pub use events::{EventLog, Level};
pub use http::{Gate, Service};
pub use rules::Rules;
pub use settings::Settings;

/// Where the app keeps its data when `XDG_DATA_HOME` is set (the default on Linux).
///
/// `$XDG_DATA_HOME/genrl-downloader/` holds `genrl.db`, cached posters and backups.
pub fn default_data_dir() -> std::path::PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return std::path::PathBuf::from(xdg).join("genrl-downloader");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("genrl-downloader")
}

/// Path of the SQLite database inside the data folder.
pub fn default_db_path() -> std::path::PathBuf {
    default_data_dir().join("genrl.db")
}
