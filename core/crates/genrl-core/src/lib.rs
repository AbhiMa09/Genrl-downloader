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
//! * [`matching`] — the title-matching engine: name sets, the search order,
//!   the accept/ask decision (phase 3).
//! * [`season`] — the season chain and the pure decision function that says
//!   what a release is good for (phase 3).
//! * [`names`] — a show's name set, stored and searched in priority order.
//! * [`questions`] — the Inbox: question objects, payloads and answers.
//! * [`jobs`], [`queue`], [`worker`] — the job state machine, the scheduler
//!   that runs it, and the crash-recovery reconcile step (phase 4).
//! * [`planner`] — turning the MAL list into episode rows and jobs, with the
//!   first-sync preview and the daily cap.
//! * [`mal`] — the real MAL sync (phase 5): OAuth with PKCE, the list and its
//!   diff, and the season chain from MAL's sequel/prequel relations.
//! * [`qbit`] — the qBittorrent WebUI client (phase 2).
//! * [`secrets`] — credentials, in the system keyring only.
//!
//! No other module talks to the network directly.

pub mod db;
pub mod clock;
pub mod error;
pub mod events;
pub mod filename;
pub mod http;
pub mod jobs;
pub mod library;
pub mod logging;
pub mod mal;
pub mod matching;
pub mod models;
pub mod names;
pub mod nyaa;
pub mod planner;
pub mod qbit;
pub mod questions;
pub mod queue;
pub mod rules;
pub mod score;
pub mod season;
pub mod secrets;
pub mod settings;
pub mod worker;
pub mod locations;
pub mod metadata;

pub use db::Db;
pub use error::{Error, Result};
pub use events::{EventLog, Level};
pub use http::{Gate, Service};
pub use library::{LibraryFinalizer, RescanReport, rescan_library, rewrite_show_folder_metadata};
pub use locations::{LocationCheck, LocationsReport, get_locations, set_location, check_directory};
pub use metadata::{fetch_show_metadata, process_metadata_queue, queue_stale_metadata, MetadataOutcome};
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
