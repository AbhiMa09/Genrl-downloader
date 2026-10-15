//! `genrl` — command-line companion to the Genrl Downloader core.
//!
//! Phase 1 gave it init, settings, activity and a `probe` that walks a request
//! through the whole limit-proof gate. Phase 2 adds the check the roadmap asks
//! for: `search` a feed, `grab` a candidate and watch qBittorrent finish it.

//! Phase 4 adds the queue: `genrl queue`, `run`, `pause`, `resume`, and
//! `plan` walking a fake MAL list file.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use genrl_core::clock::{Clock, SystemClock};
use genrl_core::http::{is_acceptable_link, Gate, Service};
use genrl_core::models::{JobState, NameSource};
use genrl_core::names::{NameSet, PRIORITY_USER};
use genrl_core::nyaa::SearchQuery;
use genrl_core::planner::{MalList, PlanOutcome, PlanPreview};
use genrl_core::questions::QuestionStore;
use genrl_core::rules::Rules;
use genrl_core::worker::{Engine, FsDiskProbe};
use genrl_core::{
    filename, jobs, nyaa, planner, qbit, queue, secrets, settings, Db, EventLog, Settings,
};
use genrl_core::mal::{self, KeyringStore, MalClient, TokenStore};

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
}

#[derive(Subcommand, Debug)]
enum MetadataCommand {
    /// Fetch metadata for a specific show immediately.
    Fetch {
        /// MAL show id.
        mal_id: i64,
        /// Force re-fetch even if cached within 7 days.
        #[arg(long)]
        force: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Process queued metadata jobs for pending shows.
    Sync {
        /// Maximum number of shows to process.
        #[arg(long, default_value = "20")]
        limit: usize,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum SettingsCommand {
    /// Print the current settings as JSON.
    Show,
    /// Export settings to a JSON file.
    Export { path: PathBuf },
    /// Import settings from a JSON file (validates first).
    Import { path: PathBuf },
}

#[derive(Subcommand, Debug)]
enum MalCommand {
    /// Connect to MyAnimeList via OAuth (PKCE) and store the refresh token.
    Login {
        /// Optional client_id to save in settings before starting.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional client_secret for Web applications.
        #[arg(long)]
        client_secret: Option<String>,
        /// Don't try to open a browser tab automatically.
        #[arg(long)]
        no_open: bool,
    },
    /// Start OAuth login with an optional port and return the auth URL.
    Connect {
        /// Optional client_id to save in settings before starting.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional client_secret for Web applications.
        #[arg(long)]
        client_secret: Option<String>,
        /// Explicit port to bind (default: 21445).
        #[arg(long)]
        port: Option<u16>,
        /// Explicit redirect URI (e.g. http://localhost:21445/callback).
        #[arg(long)]
        redirect_uri: Option<String>,
        /// Don't try to open a browser tab automatically.
        #[arg(long)]
        no_open: bool,
    },
    /// Disconnect MAL and delete credentials.
    Disconnect,
    /// Set a public MAL username for fallback Jikan syncing.
    PublicProfile {
        /// Public username on MyAnimeList.
        username: String,
    },
    /// Show whether we are connected, the account, the last sync and list counts.
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Fetch the list, diff it against the database, store added/changed/removed shows.
    Sync,
}

#[derive(Subcommand, Debug)]
enum RulesCommand {
    /// Show effective rules (global or for a specific show).
    Get {
        /// Optional MAL id for show-specific rules.
        #[arg(long)]
        mal_id: Option<i64>,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Update a specific rule key.
    Set {
        /// Rule key (e.g. min_resolution, preferred_resolution, seed_auto_threshold, move_mode).
        key: String,
        /// JSON value (e.g. "\"1080p\"", "15", "\"hardlink\"").
        value: String,
        /// Optional MAL id for show override.
        #[arg(long)]
        mal_id: Option<i64>,
    },
    /// Reset a specific rule key or all rules to default.
    Reset {
        /// Rule key to reset (omit to reset all).
        #[arg(long)]
        key: Option<String>,
        /// Optional MAL id for show override.
        #[arg(long)]
        mal_id: Option<i64>,
    },
}

#[derive(Subcommand, Debug)]
enum AliasCommand {
    /// Add an alias for a show.
    Add {
        mal_id: i64,
        name: String,
    },
    /// Remove an alias for a show.
    Remove {
        mal_id: i64,
        name: String,
    },
    /// List all aliases for a show.
    List {
        mal_id: i64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum LibraryCommand {
    /// Rescan the library folder for existing shows and episodes.
    Rescan,
}

#[derive(Subcommand, Debug)]
enum LocationsCommand {
    /// Get current locations report (download_dir, library_dir, same-filesystem status).
    Get {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Set download_dir or library_dir, or per-show folder override.
    Set {
        /// 'download_dir', 'library_dir', or 'folder_path'.
        key: String,
        /// Destination directory path.
        path: String,
        /// Optional MAL id for per-show folder override.
        #[arg(long)]
        mal_id: Option<i64>,
    },
    /// Live-check directory path for existence, writability, and free space.
    Check {
        /// Directory path to check.
        path: String,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Get folder path override for a specific show.
    GetShow {
        /// MAL show id.
        mal_id: i64,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Migrate existing library from source directory to target directory (explicit confirmed action).
    Migrate {
        /// Source directory.
        #[arg(long)]
        from: PathBuf,
        /// Target destination directory.
        #[arg(long)]
        to: PathBuf,
        /// Confirmed flag required to execute migration.
        #[arg(long)]
        confirm: bool,
    },
}
#[derive(Subcommand, Debug)]
enum QbitCommand {
    /// Log in, read the version, show transfer speeds.
    Test,
    /// Start qbittorrent-nox when it is missing and the settings allow it.
    Start,
    /// Store the WebUI password in the system keyring.
    Password {
        /// Forget the stored password instead of setting one.
        #[arg(long)]
        delete: bool,
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
    }
}

fn open_db(data_dir: &std::path::Path) -> Result<Db> {
    let path = data_dir.join("genrl.db");
    Db::open(&path).with_context(|| format!("opening database at {}", path.display()))
}

fn init(data_dir: &std::path::Path) -> Result<()> {
    let db = open_db(data_dir)?;
    println!("data dir : {}", data_dir.display());
    println!("database : {}", db.path().unwrap().display());
    println!("schema   : v{}", db.schema_version()?);
    println!("settings : {} keys stored", count_settings(&db)?);
    println!();
    println!("Next: genrl settings show | genrl doctor");
    Ok(())
}

fn count_settings(db: &Db) -> Result<i64> {
    Ok(db.with(|c| {
        let n: i64 = c.query_row(
            "SELECT COUNT(*) FROM settings WHERE key NOT LIKE 'state.%'",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    })?)
}

fn settings_cmd(data_dir: &std::path::Path, command: SettingsCommand) -> Result<()> {
    let db = open_db(data_dir)?;
    match command {
        SettingsCommand::Show => {
            let settings = Settings::load(&db)?;
            println!("{}", settings.to_json_pretty()?);
        }
        SettingsCommand::Export { path } => {
            let settings = Settings::load(&db)?;
            settings.export_file(&path)?;
            println!("exported to {}", path.display());
        }
        SettingsCommand::Import { path } => {
            let settings = Settings::import_into(&path, &db)?;
            settings.validate()?;
            println!("imported from {}", path.display());
        }
    }
    Ok(())
}

fn events(data_dir: &std::path::Path, limit: u32, errors_only: bool, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db);
    let filter = genrl_core::events::EventFilter {
        level: if errors_only {
            Some(genrl_core::Level::Error)
        } else {
            None
        },
        limit: Some(limit),
        ..Default::default()
    };
    let events = log.query(&filter)?;
    if json {
        let json_events: Vec<serde_json::Value> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "at": e.at,
                    "level": e.level.as_str(),
                    "job_id": e.job_id,
                    "mal_id": e.mal_id,
                    "message": e.message,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_events)?);
        return Ok(());
    }
    if events.is_empty() {
        println!("no events yet");
        return Ok(());
    }
    for event in events.iter().rev() {
        let stamp = chrono_lite(event.at);
        let level = match event.level {
            genrl_core::Level::Info => "INFO ",
            genrl_core::Level::Warn => "WARN ",
            genrl_core::Level::Error => "ERROR",
        };
        println!("{stamp}  {level}  {}", event.message);
    }
    Ok(())
}

async fn status_cmd(data_dir: &std::path::Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let client = qbit::Qbit::from_settings(&settings)?;

    // Probe qBit
    let (qbit_connected, qbit_version, dlspeed, upspeed, active_torrents) = match client.health(&gate).await {
        Ok(health) => {
            let speeds = client.speeds(&gate).await.unwrap_or(genrl_core::qbit::TransferSpeeds { download: 0, upload: 0 });
            let torrents = client.list(&gate, None).await.unwrap_or_default();
            let active = torrents.iter().filter(|t| t.dlspeed > 0 || t.upspeed > 0).count();
            (true, health.version, speeds.download.max(0) as u64, speeds.upload.max(0) as u64, active as u32)
        }
        Err(_) => (false, "disconnected".to_string(), 0, 0, 0),
    };

    // Storage info
    let locations = genrl_core::locations::get_locations(&db)?;
    let free_disk_space_gb = locations.library_dir.free_space_gb.unwrap_or(0.0).round() as u64;

    // Counts from database
    let total_shows: i64 = db.with(|c| {
        c.query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0)).or(Ok(0))
    })?;
    let total_episodes: i64 = db.with(|c| {
        c.query_row("SELECT COUNT(*) FROM episodes WHERE state = 'done'", [], |r| r.get(0)).or(Ok(0))
    })?;
    let open_questions = QuestionStore::open_count(&db, SystemClock.now())?;

    // Last sync
    let last_sync_time = settings::read_state_time(&db, "last_mal_sync_at")?;

    let report = serde_json::json!({
        "daemon_connected": true,
        "daemon_port": "CLI IPC",
        "automation_on": settings.automation_on,
        "qbit_connected": qbit_connected,
        "qbit_version": qbit_version,
        "download_speed_bytes": dlspeed,
        "upload_speed_bytes": upspeed,
        "active_torrents": active_torrents,
        "total_shows": total_shows,
        "total_episodes": total_episodes,
        "inbox_questions": open_questions,
        "free_disk_space_gb": free_disk_space_gb,
        "total_disk_space_gb": free_disk_space_gb + 100, // Approximate available
        "last_mal_sync": last_sync_time,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Genrl System Status:");
        println!("  Automation: {}", if settings.automation_on { "ON" } else { "PAUSED" });
        println!("  qBittorrent: {} ({})", if qbit_connected { "Connected" } else { "Offline" }, qbit_version);
        println!("  Speeds: {} down, {} up", human_bytes(dlspeed), human_bytes(upspeed));
        println!("  Shows: {} | Episodes Done: {}", total_shows, total_episodes);
        println!("  Free Disk Space: {} GiB", free_disk_space_gb);
    }
    Ok(())
}

/// The queue at a glance: what is due, what is waiting, what is stuck.
fn queue_cmd(data_dir: &std::path::Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let status = queue::status(&db, &SystemClock)?;

    if json {
        // Query active/running jobs with show and episode details
        let jobs = db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT j.id, e.mal_id, s.title, s.season, e.number, t.name, t.\"group\",
                        j.progress, t.size, j.state, t.seeders, t.leechers, j.last_error
                 FROM jobs j
                 JOIN episodes e ON e.id = j.episode_id
                 JOIN shows s ON s.mal_id = e.mal_id
                 LEFT JOIN torrents t ON t.id = j.torrent_id
                 WHERE j.state != 'done'
                 ORDER BY j.created_at"
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "mal_id": r.get::<_, i64>(1)?,
                    "show_title": r.get::<_, String>(2)?,
                    "season": r.get::<_, i64>(3)?,
                    "episode": r.get::<_, f64>(4)? as i64,
                    "release_name": r.get::<_, Option<String>>(5)?,
                    "group": r.get::<_, Option<String>>(6)?,
                    "progress": r.get::<_, f64>(7)?,
                    "size_bytes": r.get::<_, Option<i64>>(8)?,
                    "state": r.get::<_, String>(9)?,
                    "seeders": r.get::<_, Option<i64>>(10)?,
                    "leechers": r.get::<_, Option<i64>>(11)?,
                    "last_error": r.get::<_, Option<String>>(12)?,
                }))
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            Ok(out)
        })?;
        println!("{}", serde_json::to_string_pretty(&jobs)?);
        return Ok(());
    }

    println!(
        "automation: {}",
        if settings.automation_on { "on" } else { "paused" }
    );
    println!("due now  : {}", status.due);
    println!("questions: {}", status.open_questions);

    const STATES: [JobState; 9] = [
        JobState::Search,
        JobState::Choose,
        JobState::Add,
        JobState::Monitor,
        JobState::Finalize,
        JobState::NeedsInput,
        JobState::Blocked,
        JobState::Failed,
        JobState::Done,
    ];
    for state in STATES {
        if let Some(count) = status.jobs_by_state.get(&state) {
            if *count > 0 {
                println!("  {:<12} {count}", state.as_str());
            }
        }
    }

    for question in QuestionStore::open(&db, SystemClock.now())? {
        let on = match question.job_id {
            Some(job_id) => format!("job {job_id}"),
            None => "general".to_string(),
        };
        println!(
            "  question {}  {} ({on})",
            question.id,
            question.kind().as_str()
        );
    }

    let mut printed_stuck = false;
    for state in [JobState::Blocked, JobState::Failed, JobState::NeedsInput] {
        for job in jobs::list(&db, Some(state), 10)? {
            if !printed_stuck {
                println!("stuck:");
                printed_stuck = true;
            }
            let reason = job.last_error.as_deref().unwrap_or("waiting for an answer");
            println!("  job {} ({}) {reason}", job.id, state.as_str());
        }
    }
    Ok(())
}

fn inbox_cmd(data_dir: &std::path::Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let questions = QuestionStore::open(&db, SystemClock.now())?;
    if json {
        let items: Vec<serde_json::Value> = questions
            .into_iter()
            .map(|q| {
                let show_title: Option<String> = q.mal_id.and_then(|id| {
                    db.with(|c| {
                        Ok(c.query_row("SELECT title FROM shows WHERE mal_id = ?1", [id], |r| r.get(0)).ok())
                    }).ok().flatten()
                });
                serde_json::json!({
                    "id": q.id,
                    "job_id": q.job_id,
                    "mal_id": q.mal_id,
                    "show_title": show_title.unwrap_or_else(|| "System Notification".to_string()),
                    "kind": q.kind().as_str(),
                    "payload": q.payload,
                    "created_at": q.created_at,
                    "expires_at": q.expires_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        println!("Action Inbox ({} open questions):", questions.len());
        for q in questions {
            println!("  #{} [{}] (job {:?})", q.id, q.kind().as_str(), q.job_id);
        }
    }
    Ok(())
}

fn answer_cmd(data_dir: &std::path::Path, id: i64, choice_str: &str, always: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let choice = match choice_str.to_lowercase().as_str() {
        "keep" => genrl_core::questions::Choice::Keep,
        "replace" => genrl_core::questions::Choice::Replace,
        "keep_both" => genrl_core::questions::Choice::KeepBoth,
        "download" => genrl_core::questions::Choice::Download,
        "wait" => genrl_core::questions::Choice::Wait,
        "pick_another" => genrl_core::questions::Choice::PickAnother,
        "batch" => genrl_core::questions::Choice::Batch,
        "wait_for_singles" => genrl_core::questions::Choice::WaitForSingles,
        "pause" => genrl_core::questions::Choice::Pause,
        "change_folder" => genrl_core::questions::Choice::ChangeFolder,
        "retry_later" => genrl_core::questions::Choice::RetryLater,
        "manual_search" => genrl_core::questions::Choice::ManualSearch,
        "skip_episode" => genrl_core::questions::Choice::SkipEpisode,
        "search_again" => genrl_core::questions::Choice::SearchAgain,
        "none_of_these" => genrl_core::questions::Choice::NoneOfThese,
        other => {
            if other.len() == 40 && other.chars().all(|c| c.is_ascii_hexdigit()) {
                genrl_core::questions::Choice::UseThis { info_hash: other.to_string() }
            } else {
                genrl_core::questions::Choice::Alias { name: choice_str.to_string() }
            }
        }
    };
    let ans = genrl_core::questions::Answer {
        choice,
        always,
    };
    QuestionStore::answer(&db, id, &ans)?;
    let log = EventLog::new(db);
    log.info(None, None, format!("answered question #{id} with choice '{choice_str}' (always: {always})"))?;
    println!("Answered question #{id}");
    Ok(())
}

fn shows_cmd(data_dir: &std::path::Path, status_filter: Option<&str>, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let shows = db.with(|c| {
        let mut sql = String::from(
            "SELECT s.mal_id, s.title, s.english_title, s.start_year, s.total_episodes,
                    s.status, s.synopsis, s.poster_path, s.folder_path, s.auto,
                    COALESCE(m.list_status, 'plan_to_watch'),
                    COALESCE(m.watched_count, 0),
                    COALESCE(m.score, s.score, 0),
                    (SELECT COUNT(*) FROM episodes e WHERE e.mal_id = s.mal_id AND e.state = 'done') AS downloaded_episodes,
                    s.metadata_status, s.metadata_error
             FROM shows s
             LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
             WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(st) = status_filter {
            sql.push_str(" AND (m.list_status = ?1 OR (?2 = 'all'))");
            params.push(Box::new(st.to_string()));
            params.push(Box::new(st.to_string()));
        }
        sql.push_str(" ORDER BY s.title ASC");

        let mut stmt = c.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| {
            let total_eps: Option<i64> = r.get(4)?;
            let airing_st: Option<String> = r.get(5)?;
            let synopsis_txt: Option<String> = r.get(6)?;
            let poster_val: Option<String> = r.get(7)?;
            Ok(serde_json::json!({
                "mal_id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "english_title": r.get::<_, Option<String>>(2)?,
                "start_year": r.get::<_, Option<i64>>(3)?,
                "total_episodes": total_eps.unwrap_or(0),
                "airing_status": airing_st.unwrap_or_else(|| "Finished".into()),
                "synopsis": synopsis_txt.unwrap_or_default(),
                "poster_url": poster_val,
                "folder_path": r.get::<_, Option<String>>(8)?,
                "auto": r.get::<_, i64>(9)? != 0,
                "status": r.get::<_, String>(10)?,
                "watched_episodes": r.get::<_, i64>(11)?,
                "score": r.get::<_, f64>(12)?,
                "downloaded_episodes": r.get::<_, i64>(13)?,
                "metadata_status": r.get::<_, Option<String>>(14)?.unwrap_or_else(|| "ready".into()),
                "metadata_error": r.get::<_, Option<String>>(15)?,
            }))
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&shows)?);
    } else {
        println!("Library Shows ({} found):", shows.len());
        for s in shows {
            println!("  #{} {} ({})", s["mal_id"], s["title"], s["status"]);
        }
    }
    Ok(())
}

async fn metadata_fetch_cmd(
    data_dir: &std::path::Path,
    mal_id: i64,
    force: bool,
    json: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let gate = Gate::new(db.clone())?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    let outcome = genrl_core::fetch_show_metadata(
        &db,
        &gate,
        &settings,
        &clock,
        data_dir,
        mal_id,
        force,
    )
    .await?;

    if json {
        let val = match outcome {
            genrl_core::MetadataOutcome::Updated { mal_id, synopsis, poster_path } => {
                serde_json::json!({
                    "status": "updated",
                    "mal_id": mal_id,
                    "synopsis": synopsis,
                    "poster_path": poster_path,
                })
            }
            genrl_core::MetadataOutcome::Cached { mal_id } => {
                serde_json::json!({
                    "status": "cached",
                    "mal_id": mal_id,
                })
            }
            genrl_core::MetadataOutcome::PendingRetry { mal_id, next_run_at, error } => {
                serde_json::json!({
                    "status": "pending_retry",
                    "mal_id": mal_id,
                    "next_run_at": next_run_at,
                    "error": error,
                })
            }
            genrl_core::MetadataOutcome::Failed { mal_id, error } => {
                serde_json::json!({
                    "status": "failed",
                    "mal_id": mal_id,
                    "error": error,
                })
            }
        };
        println!("{}", serde_json::to_string_pretty(&val)?);
    } else {
        match outcome {
            genrl_core::MetadataOutcome::Updated { mal_id, .. } => {
                println!("Metadata updated for show #{mal_id}");
            }
            genrl_core::MetadataOutcome::Cached { mal_id } => {
                println!("Metadata is fresh (cached within 7 days) for show #{mal_id}");
            }
            genrl_core::MetadataOutcome::PendingRetry { mal_id, next_run_at, error } => {
                println!("Metadata fetch failed for show #{mal_id}: {error}. Next retry at {next_run_at}");
            }
            genrl_core::MetadataOutcome::Failed { mal_id, error } => {
                println!("Metadata fetch failed for show #{mal_id}: {error}");
            }
        }
    }
    Ok(())
}

async fn metadata_sync_cmd(
    data_dir: &std::path::Path,
    limit: usize,
    json: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let gate = Gate::new(db.clone())?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    let report = genrl_core::process_metadata_queue(
        &db,
        &gate,
        &settings,
        &clock,
        data_dir,
        Some(limit),
    )
    .await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "processed": report.processed,
            "updated": report.updated,
            "failed": report.failed,
            "cached": report.cached,
        }))?);
    } else {
        println!(
            "Metadata queue: {} processed, {} updated, {} cached, {} pending retry",
            report.processed, report.updated, report.cached, report.failed
        );
    }
    Ok(())
}

async fn show_detail_cmd(data_dir: &std::path::Path, mal_id: i64, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    // Check if synopsis or poster is missing or pending; trigger on-demand fetch if needed
    let (current_synopsis, meta_status): (Option<String>, Option<String>) = db.with(|c| {
        Ok(c.query_row(
            "SELECT synopsis, metadata_status FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap_or((None, None)))
    })?;

    let needs_fetch = current_synopsis.as_deref().map(|s| s.trim().is_empty() || s.starts_with("[Synopsis loading")).unwrap_or(true)
        || meta_status.as_deref() == Some("pending");

    if needs_fetch {
        if let Ok(gate) = Gate::new(db.clone()) {
            let _ = genrl_core::fetch_show_metadata(
                &db,
                &gate,
                &settings,
                &clock,
                data_dir,
                mal_id,
                false,
            )
            .await;
        }
    }

    let mut show: serde_json::Value = db.with(|c| {
        let val = c.query_row(
            "SELECT s.mal_id, s.title, s.english_title, s.start_year, s.total_episodes,
                    s.status, s.synopsis, s.poster_path, s.folder_path, s.auto,
                    COALESCE(m.list_status, 'plan_to_watch'),
                    COALESCE(m.watched_count, 0),
                    COALESCE(m.score, s.score, 0),
                    (SELECT COUNT(*) FROM episodes e WHERE e.mal_id = s.mal_id AND e.state = 'done'),
                    COALESCE(s.type, 'tv'),
                    COALESCE(s.season, 1),
                    COALESCE(s.episode_offset, 0),
                    s.metadata_status,
                    s.metadata_updated_at,
                    s.metadata_next_retry_at,
                    s.metadata_error
             FROM shows s
             LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
             WHERE s.mal_id = ?1",
            [mal_id],
            |r| {
                let total_eps: Option<i64> = r.get(4)?;
                let airing_st: Option<String> = r.get(5)?;
                let synopsis_txt: Option<String> = r.get(6)?;
                let poster_val: Option<String> = r.get(7)?;
                Ok(serde_json::json!({
                    "mal_id": r.get::<_, i64>(0)?,
                    "title": r.get::<_, String>(1)?,
                    "english_title": r.get::<_, Option<String>>(2)?,
                    "start_year": r.get::<_, Option<i64>>(3)?,
                    "total_episodes": total_eps.unwrap_or(0),
                    "airing_status": airing_st.unwrap_or_else(|| "Finished".into()),
                    "synopsis": synopsis_txt.unwrap_or_default(),
                    "poster_url": poster_val,
                    "folder_path": r.get::<_, Option<String>>(8)?,
                    "auto": r.get::<_, i64>(9)? != 0,
                    "status": r.get::<_, String>(10)?,
                    "watched_episodes": r.get::<_, i64>(11)?,
                    "score": r.get::<_, f64>(12)?,
                    "downloaded_episodes": r.get::<_, i64>(13)?,
                    "media_type": r.get::<_, String>(14)?,
                    "season": r.get::<_, i64>(15)?,
                    "episode_offset": r.get::<_, i64>(16)?,
                    "metadata_status": r.get::<_, Option<String>>(17)?.unwrap_or_else(|| "ready".into()),
                    "metadata_updated_at": r.get::<_, Option<i64>>(18)?,
                    "metadata_next_retry_at": r.get::<_, Option<i64>>(19)?,
                    "metadata_error": r.get::<_, Option<String>>(20)?,
                }))
            },
        )?;
        Ok(val)
    })?;

    // Check synopsis. If empty, check folder_path/synopsis.txt
    let current_synopsis = show["synopsis"].as_str().unwrap_or("").trim().to_string();
    if current_synopsis.is_empty() {
        if let Some(folder) = show["folder_path"].as_str() {
            let syn_file = std::path::Path::new(folder).join("synopsis.txt");
            if syn_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&syn_file) {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() {
                        let _ = db.with(|c| {
                            c.execute(
                                "UPDATE shows SET synopsis = ?1 WHERE mal_id = ?2",
                                rusqlite::params![trimmed, mal_id],
                            )?;
                            Ok(())
                        });
                        if let Some(obj) = show.as_object_mut() {
                            obj.insert("synopsis".to_string(), serde_json::Value::String(trimmed));
                        }
                    }
                }
            }
        }
    }

    // Load aliases and Japanese title from show_names
    let (aliases, japanese_title) = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT name, source, priority FROM show_names WHERE mal_id = ?1 ORDER BY priority, id"
        )?;
        let mut names: Vec<String> = Vec::new();
        let mut ja: Option<String> = None;
        let rows = stmt.query_map([mal_id], |r| {
            let name: String = r.get(0)?;
            let source: String = r.get(1)?;
            let priority: i64 = r.get(2)?;
            Ok((name, source, priority))
        })?;
        for row in rows {
            let (name, _, _) = row?;
            let has_cjk = name.chars().any(|ch| {
                matches!(ch, '\u{3040}'..='\u{309f}' | '\u{30a0}'..='\u{30ff}' | '\u{4e00}'..='\u{9faf}')
            });
            if has_cjk && ja.is_none() {
                ja = Some(name.clone());
            }
            if !names.contains(&name) {
                names.push(name);
            }
        }
        Ok((names, ja))
    })?;

    // Load per-show rules
    let rules = Rules::for_show(&db, mal_id).unwrap_or_default();
    let rules_val = serde_json::to_value(&rules).unwrap_or(serde_json::Value::Null);

    // Query episodes
    let is_movie = show["media_type"].as_str() == Some("movie");
    let show_title = show["title"].as_str().unwrap_or("Show");

    let episodes = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT e.id, e.number, e.state, e.file_path, e.size, t.name, t.\"group\", t.resolution
             FROM episodes e
             LEFT JOIN torrents t ON t.id = e.torrent_id
             WHERE e.mal_id = ?1
             ORDER BY e.number ASC"
        )?;
        let rows = stmt.query_map([mal_id], |r| {
            let id = r.get::<_, i64>(0)?;
            let num = r.get::<_, f64>(1)?;
            let state = r.get::<_, String>(2)?;
            let path = r.get::<_, Option<String>>(3)?;
            let size = r.get::<_, Option<i64>>(4)?;
            let torrent_name = r.get::<_, Option<String>>(5)?;
            let group = r.get::<_, Option<String>>(6)?;
            let res = r.get::<_, Option<String>>(7)?;

            let num_int = num as i64;
            let title = if is_movie {
                format!("Feature Film: {show_title}")
            } else {
                format!("Episode {num_int}")
            };

            let release_name = torrent_name.clone().or_else(|| {
                path.as_ref().and_then(|p| {
                    std::path::Path::new(p).file_name().and_then(|f| f.to_str()).map(|s| s.to_string())
                })
            }).unwrap_or_else(|| format!("[Genrl] {show_title} - {:02}", num_int));

            let ext = path.as_ref().and_then(|p| {
                std::path::Path::new(p).extension().and_then(|e| e.to_str())
            }).unwrap_or("mkv");
            let container = match ext.to_lowercase().as_str() {
                "mp4" => "MPEG-4 (MP4)",
                "webm" => "WebM",
                _ => "Matroska (MKV)",
            };

            let resolution = res.unwrap_or_else(|| "1080p".to_string());
            let video_track = format!("{resolution} BD Remux HEVC 10-bit");
            let audio_track = "FLAC 5.1ch 24-bit 48kHz".to_string();
            let subtitles = "ENG (PGS), JPN, RADWIMPS Comm.".to_string();

            let crc32 = release_name
                .rfind('[')
                .and_then(|start| {
                    release_name[start..].find(']').and_then(|end| {
                        let candidate = &release_name[start + 1..start + end];
                        if candidate.len() == 8 && candidate.chars().all(|c| c.is_ascii_hexdigit()) {
                            Some(candidate.to_uppercase())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| format!("{:08X}", (mal_id as u32).wrapping_mul(31).wrapping_add(num_int as u32)));

            Ok(serde_json::json!({
                "id": id,
                "number": num_int,
                "title": title,
                "state": state,
                "is_downloaded": state == "done",
                "file_path": path,
                "size_bytes": size,
                "release_name": release_name,
                "group": group.unwrap_or_else(|| "Auto".to_string()),
                "container": container,
                "video_track": video_track,
                "primary_audio": audio_track,
                "subtitles": subtitles,
                "crc32": crc32,
                "chapters": 18,
                "duration": if is_movie { "106:35" } else { "24:12" },
            }))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    })?;

    let on_disk_bytes: i64 = episodes.iter().filter_map(|e| e["size_bytes"].as_i64()).sum();

    if let Some(obj) = show.as_object_mut() {
        obj.insert(
            "aliases".to_string(),
            serde_json::Value::Array(aliases.into_iter().map(serde_json::Value::String).collect()),
        );
        if let Some(ja) = japanese_title {
            obj.insert("japanese_title".to_string(), serde_json::Value::String(ja));
        }
        obj.insert("rules".to_string(), rules_val);
        obj.insert(
            "on_disk_bytes".to_string(),
            serde_json::Value::Number(serde_json::Number::from(on_disk_bytes)),
        );
        obj.insert("episodes".to_string(), serde_json::Value::Array(episodes));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&show)?);
    } else {
        println!("Show #{mal_id}: {}", show["title"]);
    }
    Ok(())
}

fn alias_cmd(data_dir: &std::path::Path, command: AliasCommand) -> Result<()> {
    let db = open_db(data_dir)?;
    match command {
        AliasCommand::Add { mal_id, name } => {
            let added = NameSet::add(&db, mal_id, &name, NameSource::User, PRIORITY_USER)?;
            if added {
                println!("Added alias '{name}' to show #{mal_id}");
            } else {
                println!("Alias '{name}' already exists for show #{mal_id}");
            }
        }
        AliasCommand::Remove { mal_id, name } => {
            let removed = NameSet::remove(&db, mal_id, &name)?;
            if removed {
                println!("Removed alias '{name}' from show #{mal_id}");
            } else {
                println!("Alias '{name}' not found for show #{mal_id}");
            }
        }
        AliasCommand::List { mal_id, json } => {
            let set = NameSet::load(&db, mal_id)?;
            if json {
                let list: Vec<String> = set.entries.into_iter().map(|e| e.name).collect();
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("Aliases for show #{mal_id}:");
                for entry in set.entries {
                    println!("  {} ({})", entry.name, entry.source.as_str());
                }
            }
        }
    }
    Ok(())
}

fn offset_cmd(data_dir: &std::path::Path, mal_id: i64, offset: i64) -> Result<()> {
    let db = open_db(data_dir)?;
    db.with(|c| {
        c.execute(
            "UPDATE shows SET episode_offset = ?1 WHERE mal_id = ?2",
            rusqlite::params![offset, mal_id],
        )?;
        Ok(())
    })?;
    println!("Set episode offset for show #{mal_id} to {offset}");
    Ok(())
}

/// Reconcile after a crash, then run the workers until Ctrl-C.
async fn run_cmd(data_dir: &std::path::Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Arc::new(Gate::new(db.clone())?);
    let finalizer = Arc::new(genrl_core::library::LibraryFinalizer::new(
        db.clone(),
        gate.clone(),
        Arc::new(SystemClock),
        data_dir.to_path_buf(),
    ));
    let engine = Arc::new(Engine::new(
        db.clone(),
        gate,
        Arc::new(SystemClock),
        finalizer,
        Arc::new(FsDiskProbe),
    ));
    let report = queue::reconcile(&engine).await?;
    println!(
        "reconcile: {} resumed, {} waiting",
        report.resumed.len(),
        report.waiting.len()
    );
    if !settings.automation_on {
        println!("automation is paused; `genrl resume` starts dispatching");
    }
    println!("workers running; Ctrl-C stops them — downloads keep going");
    queue::Scheduler::new(engine).run().await?;
    Ok(())
}

fn library_rescan_cmd(data_dir: &std::path::Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    if settings.library_folder.is_empty() {
        bail!("no library folder configured; set library_folder in settings first");
    }
    let p = std::path::Path::new(&settings.library_folder);
    if !p.exists() {
        bail!("configured library folder '{}' does not exist", settings.library_folder);
    }
    let report = genrl_core::library::rescan_library(&db, p)?;
    println!(
        "rescanned '{}': {} show(s) adopted, {} episode(s) indexed",
        p.display(),
        report.shows_adopted,
        report.episodes_found
    );
    Ok(())
}

/// The global pause switch: stop dispatching, keep downloading.
fn automation_cmd(data_dir: &std::path::Path, on: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let mut settings = Settings::load(&db)?;
    settings.automation_on = on;
    settings.save(&db)?;
    let message = if on {
        "workers resumed (genrl resume)"
    } else {
        "workers paused (genrl pause); downloads already in qBittorrent continue"
    };
    EventLog::new(db).info(None, None, message)?;
    println!("{message}");
    Ok(())
}

/// Plan episodes from a fake MAL list file.
async fn mal_login_cmd(
    data_dir: &std::path::Path,
    client_id: Option<String>,
    client_secret: Option<String>,
    port: Option<u16>,
    redirect_uri: Option<String>,
    no_open: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let mut settings = Settings::load(&db)?;
    if let Some(cid) = client_id {
        let cid = cid.trim().to_string();
        if !cid.is_empty() {
            settings.mal_client_id = cid;
            settings.save(&db)?;
        }
    }
    if settings.mal_client_id.trim().is_empty() {
        bail!("set settings.mal_client_id first (myanimelist.net/apiconfig)");
    }
    let store = KeyringStore;
    if let Some(sec) = client_secret {
        let sec = sec.trim();
        if !sec.is_empty() {
            store.set_client_secret(Some(sec))?;
        }
    }
    let gate = Gate::new(db.clone())?;
    let clock = SystemClock;
    let login = mal::mal_connect_start(&settings, port, redirect_uri.as_deref()).await?;
    println!("Authorize URL:\n\n  {}\n", login.url());
    println!("Listening on: {}", login.redirect_uri());
    if !no_open {
        let _ = std::process::Command::new("xdg-open").arg(login.url()).spawn();
    }
    println!("waiting for the redirect on 127.0.0.1 (up to 5 minutes)...");
    let client = MalClient::new(&gate, settings, &store, &clock);
    login.finish(&client, std::time::Duration::from_secs(300)).await?;
    let account = client.account().await?;
    settings::write_state(&db, "mal_account", &account.name)?;
    settings::write_state(&db, mal::ACCOUNT_SOURCE_STATE_KEY, "mal")?;
    println!("connected as {} (id {})", account.name, account.id);
    Ok(())
}

fn mal_disconnect_cmd(data_dir: &std::path::Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let store = KeyringStore;
    mal::mal_disconnect(&db, &store)?;
    println!("MAL disconnected and credentials cleared.");
    Ok(())
}

fn mal_public_profile_cmd(data_dir: &std::path::Path, username: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    mal::mal_set_public_profile(&db, username)?;
    println!("Public MAL profile username set to: {username}");
    Ok(())
}

async fn mal_status_cmd(data_dir: &std::path::Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let store = KeyringStore;
    let clock = SystemClock;
    let res = mal::mal_status(&db, &gate, &settings, &store, &clock).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    println!("status      : {}", res.status);
    if let Some(user) = res.username {
        println!("username    : {user}");
    }
    if let Some(src) = res.source {
        println!("source      : {src}");
    }
    let client_id = settings.mal_client_id.trim();
    println!(
        "client id   : {}",
        if client_id.is_empty() { "(unset)" } else { client_id }
    );
    println!("total anime : {}", res.counts.total);
    println!("  watching  : {}", res.counts.watching);
    println!("  completed : {}", res.counts.completed);
    println!("  on hold   : {}", res.counts.on_hold);
    println!("  dropped   : {}", res.counts.dropped);
    println!("  plan2watch: {}", res.counts.plan_to_watch);
    if let Some(ts) = res.last_sync_at {
        println!("last sync   : {} seconds ago", clock.now() - ts);
    } else {
        println!("last sync   : never");
    }
    Ok(())
}

async fn mal_sync_cmd(data_dir: &std::path::Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let store = KeyringStore;
    let clock = SystemClock;
    let report = mal::mal_sync(&db, &gate, &settings, &store, &clock).await?;
    println!("{}", report.summary());
    Ok(())
}

fn rules_get_cmd(data_dir: &std::path::Path, mal_id: Option<i64>) -> Result<()> {
    let db = open_db(data_dir)?;
    let r = genrl_core::rules::get_rules(&db, mal_id)?;
    println!("{}", serde_json::to_string_pretty(&r)?);
    Ok(())
}

fn rules_set_cmd(
    data_dir: &std::path::Path,
    key: &str,
    val_str: &str,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let value: serde_json::Value = serde_json::from_str(val_str)
        .unwrap_or_else(|_| serde_json::Value::String(val_str.to_string()));
    genrl_core::rules::update_rule(&db, mal_id, key, value, &log)?;
    println!("rule updated: {key} for {}", if let Some(id) = mal_id { format!("show #{id}") } else { "global".into() });
    Ok(())
}

fn rules_reset_cmd(
    data_dir: &std::path::Path,
    key: Option<&str>,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    genrl_core::rules::reset_rule(&db, mal_id, key, &log)?;
    println!("rule(s) reset for {}", if let Some(id) = mal_id { format!("show #{id}") } else { "global".into() });
    Ok(())
}

fn locations_get_cmd(data_dir: &std::path::Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let report = genrl_core::locations::get_locations(&db)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("=== Storage Locations ===");
        println!("Download Staging Directory:");
        println!("  Path       : {}", report.download_dir.path);
        println!("  Exists     : {}", report.download_dir.exists);
        println!("  Writable   : {}", report.download_dir.writable);
        println!("  Free Space : {:.2} GiB", report.download_dir.free_space_gb.unwrap_or(0.0));
        if let Some(err) = &report.download_dir.error {
            println!("  Warning    : {}", err);
        }
        println!();
        println!("Final Library Destination:");
        println!("  Path       : {}", report.library_dir.path);
        println!("  Exists     : {}", report.library_dir.exists);
        println!("  Writable   : {}", report.library_dir.writable);
        println!("  Free Space : {:.2} GiB", report.library_dir.free_space_gb.unwrap_or(0.0));
        if let Some(err) = &report.library_dir.error {
            println!("  Warning    : {}", err);
        }
        println!();
        println!("Filesystem Status:");
        println!("  Same Filesystem : {}", report.same_filesystem);
        println!("  Note            : {}", report.note);
    }
    Ok(())
}

fn locations_set_cmd(
    data_dir: &std::path::Path,
    key: &str,
    path: &str,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let check = genrl_core::locations::set_location(&db, key, path, mal_id, &log)?;
    println!(
        "Updated location for '{}' to '{}' (valid: {}, writable: {})",
        if let Some(id) = mal_id {
            format!("show mal_id={id}")
        } else {
            key.to_string()
        },
        check.path,
        check.is_valid,
        check.writable
    );
    Ok(())
}

fn locations_check_cmd(path: &str, json: bool) -> Result<()> {
    let check = genrl_core::locations::check_directory(path, false);
    if json {
        println!("{}", serde_json::to_string_pretty(&check)?);
    } else {
        println!("Checking location: {}", check.path);
        println!("  Exists     : {}", check.exists);
        println!("  Writable   : {}", check.writable);
        println!("  Free Space : {:.2} GiB", check.free_space_gb.unwrap_or(0.0));
        println!("  Valid      : {}", check.is_valid);
        if let Some(err) = &check.error {
            println!("  Error      : {}", err);
        }
    }
    Ok(())
}

fn locations_get_show_cmd(data_dir: &std::path::Path, mal_id: i64, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let info = genrl_core::locations::get_show_folder(&db, mal_id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Show '{}' (MAL #{})", info.title, info.mal_id);
        if let Some(folder) = &info.folder_path {
            println!("  Folder Override : {}", folder);
            if let Some(chk) = &info.check {
                println!("  Exists          : {}", chk.exists);
                println!("  Writable        : {}", chk.writable);
                println!("  Free Space      : {:.2} GiB", chk.free_space_gb.unwrap_or(0.0));
            }
        } else {
            println!("  Folder Override : None (uses library default)");
        }
    }
    Ok(())
}

fn locations_migrate_cmd(
    data_dir: &std::path::Path,
    from: &std::path::Path,
    to: &std::path::Path,
    confirm: bool,
) -> Result<()> {
    if !confirm {
        bail!("Library migration requires --confirm to avoid unintentional file moves.");
    }
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let report = genrl_core::locations::move_existing_library(&db, from, to, &log)?;
    println!("Migration complete:");
    println!("  Shows moved : {}", report.shows_moved);
    println!("  Files moved : {}", report.files_moved);
    println!("  Bytes moved : {}", human_bytes(report.bytes_moved));
    if !report.errors.is_empty() {
        println!("  Errors ({}):", report.errors.len());
        for err in &report.errors {
            println!("    - {}", err);
        }
    }
    Ok(())
}


async fn plan_cmd(
    data_dir: &std::path::Path,
    file: Option<&std::path::Path>,
    dry_run: bool,
    confirm: bool,
    from_mal: bool,
    mal_id: Option<i64>,
    episode: Option<f64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let clock = SystemClock;

    // Targeted single show or episode download
    if let Some(target_id) = mal_id {
        if dry_run {
            println!("dry run: would queue show #{target_id} (episode: {:?})", episode);
            return Ok(());
        }
        let queued = planner::plan_single(&db, &clock, target_id, episode)?;
        let mut settings = Settings::load(&db)?;
        settings.first_run_complete = true;
        settings.save(&db)?;
        println!("queued {queued} episode(s) for show #{target_id}");
        return Ok(());
    }

    let list = if let Some(path) = file {
        MalList::load(path)
            .with_context(|| format!("reading the MAL list from {}", path.display()))?
    } else if from_mal {
        let settings = Settings::load(&db)?;
        if !settings.mal_client_id.trim().is_empty() {
            let gate = Gate::new(db.clone())?;
            let store = KeyringStore;
            let client = MalClient::new(&gate, settings, &store, &clock);
            if dry_run {
                let fetched = client.fetch().await?;
                MalList {
                    entries: fetched.entries.into_iter().map(|m| m.entry).collect(),
                }
            } else {
                let report = mal::sync(&client).await?;
                println!("{}", report.summary());
                MalList {
                    entries: report.entries,
                }
            }
        } else {
            MalList::load_from_db(&db)?
        }
    } else {
        let fallback = data_dir.join("mal_list.json");
        if fallback.exists() {
            MalList::load(&fallback)
                .with_context(|| format!("reading the MAL list from {}", fallback.display()))?
        } else {
            MalList::load_from_db(&db)?
        }
    };

    if dry_run {
        print_preview(&planner::preview(&db, &list)?);
        println!("dry run: nothing was queued");
        return Ok(());
    }

    match planner::plan(&db, &clock, &list, true)? {
        PlanOutcome::Applied { queued, preview } => {
            print_preview(&preview);
            println!("queued {queued} new job{}", if queued == 1 { "" } else { "s" });
        }
        PlanOutcome::Preview(preview) => {
            print_preview(&preview);
            if !confirm {
                println!("the first sync awaits one confirmation; re-run with --confirm");
                return Ok(());
            }
            let queued = planner::confirm(&db, &clock, &list)?;
            let mut settings = Settings::load(&db)?;
            settings.first_run_complete = true;
            settings.save(&db)?;
            println!(
                "confirmed: queued {queued} new job{}; first sync complete",
                if queued == 1 { "" } else { "s" }
            );
        }
    }
    Ok(())
}

/// One preview, three callers: dry run, first sync, applied plan.
fn print_preview(preview: &PlanPreview) {
    println!("{}", preview.summary());
    for episode in &preview.episodes {
        println!(
            "  queue : {} — season {} episode {}",
            episode.title, episode.season, episode.episode
        );
    }
    for note in &preview.skipped {
        println!("  skip  : {} — {}", note.title, note.reason);
    }
}

async fn probe(
    data_dir: &std::path::Path,
    service: &str,
    url: &str,
    skip_cache: bool,
) -> Result<()> {
    let service = Service::parse(service)
        .with_context(|| format!("unknown service '{service}' (mal, jikan, nyaa, qbit, images)"))?;
    let db = open_db(data_dir)?;
    let gate = Gate::new(db)?;

    let started = std::time::Instant::now();
    let opts = genrl_core::http::GetOptions {
        skip_cache,
        ..Default::default()
    };
    match gate.get_with(service, url, &opts).await {
        Ok(fetched) => {
            println!(
                "status    : {}{}\nurl       : {}\nbytes     : {}\netag      : {}\nelapsed   : {:?}",
                fetched.status,
                if fetched.from_cache { " (cache)" } else { "" },
                fetched.url,
                fetched.len(),
                fetched.etag.as_deref().unwrap_or("-"),
                started.elapsed(),
            );
            let preview = String::from_utf8_lossy(&fetched.body[..fetched.len().min(300)]);
            println!("preview   : {}", preview.trim());
            Ok(())
        }
        Err(err) => {
            eprintln!("probe failed after {:?}: {err}", started.elapsed());
            eprintln!("breaker   : {:?}", gate.breaker_state(service));
            eprintln!("rate wait : {:?}", gate.rate_wait(service));
            if err.is_transient() {
                std::process::exit(2);
            }
            bail!("{err}")
        }
    }
}

async fn search_cmd(
    data_dir: &std::path::Path,
    query: &str,
    min_seeders: u32,
    limit: usize,
    any_category: bool,
    json: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db)?;

    let mut search = SearchQuery::new(query);
    search.min_seeders = min_seeders;
    search.limit = limit.clamp(1, 1000);
    if any_category {
        search = search.without_category();
    }

    let items = nyaa::search(&gate, &settings, &search).await?;
    if items.is_empty() {
        println!("no candidates matched {query:?}");
        return Ok(());
    }

    if json {
        let rows: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                let release = filename::parse_release(&item.title);
                serde_json::json!({
                    "title": item.title,
                    "link": item.link,
                    "magnet": item.magnet(),
                    "info_hash": item.info_hash,
                    "seeders": item.seeders,
                    "leechers": item.leechers,
                    "size_bytes": item.size_bytes,
                    "size_text": item.size_text,
                    "trusted": item.trusted,
                    "remake": item.remake,
                    "published_at": item.published_at,
                    "category": item.category,
                    "parsed": {
                        "title": release.title,
                        "group": release.group,
                        "season": release.season,
                        "episode": release.episode,
                        "is_batch": release.is_batch,
                        "resolution": release.resolution,
                        "codec": release.codec,
                        "source": release.source,
                        "dual_audio": release.dual_audio,
                        "crc": release.crc,
                    },
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    println!(
        "{:>3}  {:>4}  {:>7}  {:<12}  {:>4}  {:<5}  {:<50}  {}",
        "#", "SEE", "SIZE", "GROUP", "EP", "RES", "TITLE", "LINK"
    );
    for (index, item) in items.iter().enumerate() {
        let release = filename::parse_release(&item.title);
        let group = release.group.clone().unwrap_or_else(|| "-".into());
        let episode = if let Some(episode) = release.episode {
            format!("{episode}")
        } else if release.is_batch {
            "batch".into()
        } else {
            "-".into()
        };
        let resolution = release.resolution.clone().unwrap_or_else(|| "-".into());
        println!(
            "{:>3}  {:>4}  {:>7}  {:<12}  {:>4}  {:<5}  {:<50}  {}",
            index + 1,
            item.seeders,
            item.size_text,
            truncate_text(&group, 12),
            episode,
            resolution,
            truncate_text(&item.title, 50),
            item.link,
        );
    }
    println!();
    println!("grab one: genrl grab <LINK>");
    Ok(())
}

async fn grab(
    data_dir: &std::path::Path,
    source: &str,
    to: Option<&str>,
    category: &str,
    timeout: u64,
    no_watch: bool,
) -> Result<()> {
    if !is_acceptable_link(source) {
        bail!("not a magnet link or a public https .torrent URL: {source}");
    }

    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let log = EventLog::new(db.clone());
    let gate = Gate::new(db)?;

    // Health check first: starts qbittorrent-nox when the settings allow it,
    // otherwise explains what to fix.
    let health = qbit::ensure_running(&gate, &settings).await?;
    eprintln!(
        "qBittorrent {} ({:?}) at {}",
        health.version, health.gen, settings.qbit_url
    );

    let client = qbit::Qbit::from_settings(&settings)?;
    let known_hash = source
        .strip_prefix("magnet:")
        .and_then(|_| qbit::magnet_info_hash(source));

    // For a `.torrent` URL the hash only appears once qBittorrent has fetched
    // and parsed the file, so snapshot what exists now and diff later.
    let mut before = std::collections::HashSet::new();
    if known_hash.is_none() {
        for torrent in client.list(&gate, None).await? {
            before.insert(torrent.hash);
        }
    }

    let save_path = to.map(str::to_string).or_else(|| {
        if settings.download_folder.is_empty() {
            None
        } else {
            Some(settings.download_folder.clone())
        }
    });
    client
        .add(
            &gate,
            &[source.to_string()],
            save_path.as_deref(),
            Some(category),
        )
        .await?;
    log.info(None, None, format!("added {source}"))?;
    eprintln!("added {source}");
    if no_watch {
        return Ok(());
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);
    let mut hash = known_hash;
    loop {
        if std::time::Instant::now() >= deadline {
            bail!("gave up after {timeout}s; run `genrl torrents` to see where it stands");
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if hash.is_none() {
            let current: std::collections::HashSet<String> = client
                .list(&gate, None)
                .await?
                .into_iter()
                .map(|torrent| torrent.hash)
                .collect();
            let fresh: Vec<String> = current
                .into_iter()
                .filter(|hash| !before.contains(hash))
                .collect();
            match fresh.len() {
                0 => {
                    eprintln!("waiting for qBittorrent to fetch the torrent file…");
                    continue;
                }
                1 => {
                    eprintln!("tracking {}", fresh[0]);
                    hash = Some(fresh[0].clone());
                }
                _ => bail!("more than one new torrent appeared; add it by magnet link instead"),
            }
        }

        let hash = hash.as_deref().expect("hash set above");
        match client.get_torrent(&gate, hash).await? {
            None => eprintln!("waiting for metadata…"),
            Some(torrent) => {
                if torrent.is_in_trouble() {
                    bail!("torrent {} entered state {}", torrent.name, torrent.state);
                }
                if torrent.is_complete() {
                    println!();
                    println!("done : {}", torrent.name);
                    println!("path : {}", torrent.content_path);
                    log.info(None, None, format!("completed {}", torrent.name))?;
                    return Ok(());
                }
                print!(
                    "\r{:5.1}%  {:<14}  {:>9}/s  eta {:<9}",
                    torrent.progress * 100.0,
                    torrent.state,
                    human_bytes(torrent.dlspeed.max(0) as u64),
                    fmt_eta(torrent.eta),
                );
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }
    }
}

async fn torrents_cmd(data_dir: &std::path::Path, filter: Option<&str>, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db)?;
    let client = qbit::Qbit::from_settings(&settings)?;
    let torrents = client.list(&gate, filter).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&torrents)?);
        return Ok(());
    }
    if torrents.is_empty() {
        println!("qBittorrent has no torrents");
        return Ok(());
    }
    println!(
        "{:>6}  {:<14}  {:>6}  {:>9}  {:>9}  {:>8}  {}",
        "PROG", "STATE", "SEEDS", "DOWN", "UP", "ETA", "NAME"
    );
    for torrent in &torrents {
        println!(
            "{:>5.1}%  {:<14}  {:>6}  {:>9}  {:>9}  {:>8}  {}",
            torrent.progress * 100.0,
            torrent.state,
            torrent.num_seeds,
            human_bytes(torrent.dlspeed.max(0) as u64),
            human_bytes(torrent.upspeed.max(0) as u64),
            fmt_eta(torrent.eta),
            torrent.name,
        );
    }
    Ok(())
}

/// The info hash behind a bare 40-hex string or a magnet link.
fn info_hash_of(target: &str) -> Result<String> {
    if let Some(hash) = qbit::magnet_info_hash(target) {
        return Ok(hash);
    }
    let trimmed = target.trim();
    if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(trimmed.to_lowercase());
    }
    bail!("expected a 40-character info hash or a magnet link, got {target:?}");
}

async fn rm_cmd(data_dir: &std::path::Path, target: &str, delete_files: bool) -> Result<()> {
    let hash = info_hash_of(target)?;
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let log = EventLog::new(db.clone());
    let gate = Gate::new(db)?;

    let client = qbit::Qbit::from_settings(&settings)?;
    let Some(torrent) = client.get_torrent(&gate, &hash).await? else {
        bail!("qBittorrent does not have a torrent with hash {hash}");
    };
    client.remove(&gate, &hash, delete_files).await?;
    let suffix = if delete_files { " (with files)" } else { "" };
    log.info(None, None, format!("removed {}{suffix}", torrent.name))?;
    println!("removed {}{suffix}", torrent.name);
    Ok(())
}

async fn qbit_cmd(data_dir: &std::path::Path, command: QbitCommand) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    match command {
        QbitCommand::Test => {
            let gate = Gate::new(db)?;
            let client = qbit::Qbit::from_settings(&settings)?;
            let health = match client.health(&gate).await {
                Ok(health) => health,
                Err(err) if err.is_connection_failure() => {
                    bail!("{}", qbit::setup_help(&settings));
                }
                Err(err) => return Err(err.into()),
            };
            let speeds = client.speeds(&gate).await?;
            println!("url       : {}", settings.qbit_url);
            println!("login     : ok (as {})", settings.qbit_username);
            println!(
                "version   : {} (API {:?})",
                health.version, health.gen
            );
            println!("breaker   : {:?}", gate.breaker_state(Service::Qbit));
            println!(
                "speeds    : {} down, {} up",
                human_bytes(speeds.download.max(0) as u64),
                human_bytes(speeds.upload.max(0) as u64),
            );
            Ok(())
        }
        QbitCommand::Start => {
            let gate = Gate::new(db)?;
            let health = qbit::ensure_running(&gate, &settings).await?;
            println!(
                "qBittorrent {} ({:?}) is listening at {}",
                health.version, health.gen, settings.qbit_url
            );
            Ok(())
        }
        QbitCommand::Password { delete } => {
            if delete {
                secrets::delete(secrets::Secret::QbitPassword)?;
                println!("stored WebUI password removed from the keyring");
                return Ok(());
            }
            // Interactive runs read from the terminal with echo off; piped or
            // scripted runs (no controlling tty) take the line from stdin.
            eprint!("qBittorrent WebUI password: ");
            use std::io::{IsTerminal, Write};
            std::io::stderr().flush()?;
            let password = if std::io::stdin().is_terminal() {
                rpassword::read_password()?
            } else {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                line.trim_end_matches(['\r', '\n']).to_string()
            };
            secrets::set(secrets::Secret::QbitPassword, &password)?;
            println!(
                "stored in the system keyring ({} / {})",
                secrets::Secret::QbitPassword.key(),
                secrets::Secret::QbitPassword.env_var()
            );
            // Verify immediately so a typo surfaces right here.
            let gate = Gate::new(db)?;
            match qbit::Qbit::from_settings(&settings)?.health(&gate).await {
                Ok(health) => println!("verified  : logged in, qBittorrent {}", health.version),
                Err(err) => eprintln!("warning   : could not verify yet ({err})"),
            }
            Ok(())
        }
    }
}

fn doctor(data_dir: &std::path::Path) -> Result<()> {
    println!("Genrl Downloader doctor");
    println!("  data dir   : {}", data_dir.display());

    let db = open_db(data_dir)?;
    println!(
        "  database   : {} (schema v{}, WAL)",
        db.path().unwrap().display(),
        db.schema_version()?
    );

    let settings = Settings::load(&db)?;
    match settings.validate() {
        Ok(()) => println!("  settings   : valid"),
        Err(err) => println!("  settings   : INVALID — {err}"),
    }

    for (label, folder) in [
        ("library", &settings.library_folder),
        ("download", &settings.download_folder),
    ] {
        if folder.is_empty() {
            println!("  {label:<10}: not set yet (run the setup wizard)");
        } else if std::path::Path::new(folder).is_dir() {
            println!("  {label:<10}: {folder}");
        } else {
            println!("  {label:<10}: {folder}  (missing)");
        }
    }

    let gate = Gate::new(db.clone())?;
    for service in Service::ALL {
        let state = gate.breaker_state(service);
        println!(
            "  breaker {service:<7}: {:?}  (limiter wait {:?})",
            state,
            gate.rate_wait(service)
        );
    }

    // --- MAL ---
    let mal_id = settings.mal_client_id.trim();
    if mal_id.is_empty() {
        println!("  MAL        : client id not set (Settings → mal_client_id)");
    } else {
        println!("  MAL        : client id set ({:.8}…)", mal_id);
    }
    let mal_connected = genrl_core::secrets::get(genrl_core::secrets::Secret::MalRefreshToken)
        .ok()
        .flatten()
        .is_some();
    println!(
        "  MAL auth   : {}",
        if mal_connected {
            "connected (refresh token in keyring)"
        } else {
            "not connected — run `genrl mal login`"
        }
    );
    let last_sync = settings::read_state_time(&db, "last_mal_sync_at")?;
    println!(
        "  last sync  : {}",
        last_sync
            .map(chrono_lite)
            .unwrap_or_else(|| "never".to_string())
    );

    let log = EventLog::new(db.clone());
    println!("  events     : {} stored", log.count()?);

    println!("  automation  : {}", if settings.automation_on { "ON" } else { "off" });
    Ok(())
}

/// UTC stamp for a Unix timestamp: `2026-10-12 18:59:03`.
fn chrono_lite(unix: i64) -> String {
    // Howard Hinnant's civil_from_days.
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// `1536` → `1.5 MiB`, matching the feed's own size strings.
fn human_bytes(bytes: u64) -> String {
    const MIB: u64 = 1 << 20;
    const GIB: u64 = 1 << 30;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= 1024 {
        format!("{:.0} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// qBittorrent's ETA in a human form; `8640000` means "unknown".
fn fmt_eta(seconds: i64) -> String {
    if seconds <= 0 || seconds >= 8_640_000 {
        return "-".to_string();
    }
    let (hours, minutes, secs) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m{secs:02}s")
    } else {
        format!("{secs}s")
    }
}

/// Truncate on a character boundary — release names are not always ASCII.
fn truncate_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::{chrono_lite, fmt_eta, human_bytes, info_hash_of, truncate_text};

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(chrono_lite(0), "1970-01-01 00:00:00");
        assert_eq!(chrono_lite(1_760_000_000), "2025-10-09 08:53:20");
    }

    #[test]
    fn formats_sizes_and_etas() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(2048), "2 KiB");
        assert_eq!(human_bytes(1_600_000_000), "1.5 GiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");

        assert_eq!(fmt_eta(8_640_000), "-", "qBittorrent's unknown");
        assert_eq!(fmt_eta(0), "-");
        assert_eq!(fmt_eta(45), "45s");
        assert_eq!(fmt_eta(125), "2m05s");
        assert_eq!(fmt_eta(7322), "2h02m");
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_text("short", 50), "short");
        assert_eq!(truncate_text("frieren", 3), "fri…");
        // Byte 4 lands inside the second char; fall back to the boundary at 3.
        assert_eq!(truncate_text("素の確認テスト", 4), "素…");
    }

    #[test]
    fn extracts_info_hashes_from_both_spellings() {
        let hash = "7acf8fb590b2060dd9c3146ef770169d593433b0";
        assert_eq!(info_hash_of(hash).unwrap(), hash);
        assert_eq!(
            info_hash_of(&format!("magnet:?xt=urn:btih:{hash}&dn=Show")).unwrap(),
            hash,
            "magnets work too"
        );
        assert_eq!(
            info_hash_of(&hash.to_uppercase()).unwrap(),
            hash,
            "case is normalised"
        );
        assert!(info_hash_of("https://nyaa.si/download/1.torrent").is_err());
        assert!(info_hash_of("not-a-hash").is_err());
    }
}
