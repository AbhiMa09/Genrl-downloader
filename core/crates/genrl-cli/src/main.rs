//! `genrl` — command-line companion to the Genrl Downloader core.
//!
//! Phase 1 gave it init, settings, activity and a `probe` that walks a request
//! through the whole limit-proof gate. Phase 2 adds the check the roadmap asks
//! for: `search` a feed, `grab` a candidate and watch qBittorrent finish it.
//!
//! Phase 4 adds the queue: `genrl queue`, `run`, `pause`, `resume`, and
//! `plan` walking a fake MAL list file.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
use commands::*;

#[derive(Parser, Debug)]
#[command(
    name = "genrl",
    version,
    about = "Genrl Downloader core utilities",
    long_about = None
)]
struct Cli {
    /// Data folder holding genrl.db (default: $XDG_DATA_HOME/genrl-downloader).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create the data folder and database (runs migrations).
    Init,

    /// Inspect or move the configuration.
    Settings {
        #[command(subcommand)]
        command: SettingsCommand,
    },

    /// Tail the activity log.
    Events {
        /// How many recent events to show.
        #[arg(long, default_value_t = 20)]
        limit: u32,
        /// Only show warnings and errors.
        #[arg(long)]
        errors_only: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Search Nyaa (RSS) through the gate and list download candidates.
    Search {
        /// Query text, e.g. "sousou no frieren 1080p".
        query: String,
        /// Drop candidates with fewer seeders than this.
        #[arg(long, default_value_t = 1)]
        min_seeders: u32,
        /// Rows to ask for (clamped to 1..=1000).
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Search all categories, not just English-translated anime.
        #[arg(long)]
        any_category: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Add a torrent to qBittorrent and watch it download.
    Grab {
        /// Magnet link or .torrent URL (as printed by `genrl search`).
        source: String,
        /// Save into this folder (default: the configured download folder).
        #[arg(long)]
        to: Option<String>,
        /// Tag the torrent with this category.
        #[arg(long, default_value = "genrl")]
        category: String,
        /// Give up after this many seconds of watching.
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        /// Add it and exit immediately instead of watching progress.
        #[arg(long)]
        no_watch: bool,
    },

    /// List what qBittorrent is working on.
    Torrents {
        /// qBittorrent filter: downloading, seeding, done, paused, stalled, …
        #[arg(long)]
        filter: Option<String>,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Remove a torrent from qBittorrent.
    Rm {
        /// Info hash or magnet link (as printed by `genrl search`).
        target: String,
        /// Also delete the downloaded files from disk.
        #[arg(long)]
        delete_files: bool,
    },

    /// qBittorrent connection: health, startup, stored password.
    Qbit {
        #[command(subcommand)]
        command: QbitCommand,
    },

    /// Fetch a URL through the limiter, cache, backoff and breaker.
    Probe {
        /// Service: mal, jikan, nyaa, qbit, images.
        service: String,
        url: String,
        /// Bypass the response cache.
        #[arg(long)]
        skip_cache: bool,
    },

    /// Check the local environment: database, permissions, settings.
    Doctor,

    /// Live system status (qBit speeds, connection, storage, automation).
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Show the queue: what is due, waiting, stuck, and what the Inbox holds.
    Queue {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// List open questions in the Action Inbox.
    Inbox {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Answer an open question in the Action Inbox.
    Answer {
        /// Question ID.
        id: i64,
        /// Choice key.
        choice: String,
        /// Remember choice as a permanent rule or alias.
        #[arg(long)]
        always: bool,
    },

    /// List shows and watch progress in the library.
    Shows {
        /// Optional status filter: watching, completed, on_hold, dropped, plan_to_watch.
        #[arg(long)]
        status: Option<String>,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Get detailed show information and episodes matrix.
    Show {
        /// MAL show id.
        mal_id: i64,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },

    /// Reconcile after a crash, then run the workers until Ctrl-C.
    Run,

    /// Stop dispatching new work (downloads already running keep going).
    Pause,

    /// Start dispatching again after `genrl pause`.
    Resume,

    /// Plan episodes from MAL or the local library database.
    Plan {
        /// JSON file with the fake MAL list (default: loads from database).
        #[arg(long)]
        file: Option<PathBuf>,
        /// Pull the list from MyAnimeList instead of the database.
        #[arg(long)]
        from_mal: bool,
        /// Target a specific show by MAL id.
        #[arg(long)]
        mal_id: Option<i64>,
        /// Target a specific episode number.
        #[arg(long)]
        episode: Option<f64>,
        /// Show what would be queued; touches nothing.
        #[arg(long)]
        dry_run: bool,
        /// Accept the first-sync preview and queue it.
        #[arg(long)]
        confirm: bool,
    },

    /// MyAnimeList: OAuth, status, list sync.
    Mal {
        #[command(subcommand)]
        command: MalCommand,
    },

    /// Rules management: download preferences and overrides.
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },

    /// Library management: folder rescan.
    Library {
        #[command(subcommand)]
        command: LibraryCommand,
    },

    /// Storage locations and folder management.
    Locations {
        #[command(subcommand)]
        command: LocationsCommand,
    },

    /// Manage title aliases for matching.
    Alias {
        #[command(subcommand)]
        command: AliasCommand,
    },

    /// Set episode number offset for a show.
    Offset {
        mal_id: i64,
        offset: i64,
    },

    /// Ingest and manage show metadata (Jikan/MAL, synopsis, posters, retry queue).
    Metadata {
        #[command(subcommand)]
        command: MetadataCommand,
    },

    /// Step a single job directly through the engine.
    Step {
        job_id: i64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let data_dir = cli
        .data_dir
        .clone()
        .unwrap_or_else(genrl_core::default_data_dir);

    match cli.command {
        Command::Init => init(&data_dir),
        Command::Settings { command } => settings_cmd(&data_dir, command),
        Command::Events { limit, errors_only, json } => events(&data_dir, limit, errors_only, json),
        Command::Search {
            query,
            min_seeders,
            limit,
            any_category,
            json,
        } => search_cmd(&data_dir, &query, min_seeders, limit, any_category, json).await,
        Command::Grab {
            source,
            to,
            category,
            timeout,
            no_watch,
        } => grab(&data_dir, &source, to.as_deref(), &category, timeout, no_watch).await,
        Command::Torrents { filter, json } => torrents_cmd(&data_dir, filter.as_deref(), json).await,
        Command::Rm {
            target,
            delete_files,
        } => rm_cmd(&data_dir, &target, delete_files).await,
        Command::Qbit { command } => qbit_cmd(&data_dir, command).await,
        Command::Probe {
            service,
            url,
            skip_cache,
        } => probe(&data_dir, &service, &url, skip_cache).await,
        Command::Doctor => doctor(&data_dir),
        Command::Status { json } => status_cmd(&data_dir, json).await,
        Command::Queue { json } => queue_cmd(&data_dir, json),
        Command::Inbox { json } => inbox_cmd(&data_dir, json),
        Command::Answer { id, choice, always } => answer_cmd(&data_dir, id, &choice, always),
        Command::Shows { status, json } => shows_cmd(&data_dir, status.as_deref(), json),
        Command::Show { mal_id, json } => show_detail_cmd(&data_dir, mal_id, json).await,
        Command::Run => run_cmd(&data_dir).await,
        Command::Pause => automation_cmd(&data_dir, false),
        Command::Resume => automation_cmd(&data_dir, true),
        Command::Plan {
            file,
            dry_run,
            confirm,
            from_mal,
            mal_id,
            episode,
        } => plan_cmd(&data_dir, file.as_deref(), dry_run, confirm, from_mal, mal_id, episode).await,
        Command::Mal { command } => match command {
            MalCommand::Login { client_id, client_secret, no_open } => {
                mal_login_cmd(&data_dir, client_id, client_secret, None, None, no_open).await
            }
            MalCommand::Connect { client_id, client_secret, port, redirect_uri, no_open } => {
                mal_login_cmd(&data_dir, client_id, client_secret, port, redirect_uri, no_open).await
            }
            MalCommand::Disconnect => mal_disconnect_cmd(&data_dir),
            MalCommand::PublicProfile { username } => mal_public_profile_cmd(&data_dir, &username),
            MalCommand::Status { json } => mal_status_cmd(&data_dir, json).await,
            MalCommand::Sync => mal_sync_cmd(&data_dir).await,
        },
        Command::Rules { command } => match command {
            RulesCommand::Get { mal_id, .. } => rules_get_cmd(&data_dir, mal_id),
            RulesCommand::Set { key, value, mal_id } => rules_set_cmd(&data_dir, &key, &value, mal_id),
            RulesCommand::Reset { key, mal_id } => rules_reset_cmd(&data_dir, key.as_deref(), mal_id),
        },
        Command::Library { command } => match command {
            LibraryCommand::Rescan => library_rescan_cmd(&data_dir),
        },
        Command::Locations { command } => match command {
            LocationsCommand::Get { json } => locations_get_cmd(&data_dir, json),
            LocationsCommand::Set { key, path, mal_id } => locations_set_cmd(&data_dir, &key, &path, mal_id),
            LocationsCommand::Check { path, json } => locations_check_cmd(&path, json),
            LocationsCommand::GetShow { mal_id, json } => locations_get_show_cmd(&data_dir, mal_id, json),
            LocationsCommand::Migrate { from, to, confirm } => locations_migrate_cmd(&data_dir, &from, &to, confirm),
        },
        Command::Alias { command } => alias_cmd(&data_dir, command),
        Command::Offset { mal_id, offset } => offset_cmd(&data_dir, mal_id, offset),
        Command::Metadata { command } => match command {
            MetadataCommand::Fetch { mal_id, force, json } => {
                metadata_fetch_cmd(&data_dir, mal_id, force, json).await
            }
            MetadataCommand::Sync { limit, json } => {
                metadata_sync_cmd(&data_dir, limit, json).await
            }
        },
        Command::Step { job_id } => step_cmd(&data_dir, job_id).await,
    }
}
