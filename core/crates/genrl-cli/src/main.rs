//! `genrl` — command-line companion to the Genrl Downloader core.
//!
//! Phase 1 gave it init, settings, activity and a `probe` that walks a request
//! through the whole limit-proof gate. Phase 2 adds the check the roadmap asks
//! for: `search` a feed, `grab` a candidate and watch qBittorrent finish it.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use genrl_core::http::{is_acceptable_link, Gate, Service};
use genrl_core::nyaa::SearchQuery;
use genrl_core::{filename, nyaa, qbit, secrets, settings, Db, EventLog, Settings};

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
        Command::Events { limit, errors_only } => events(&data_dir, limit, errors_only),
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

fn events(data_dir: &std::path::Path, limit: u32, errors_only: bool) -> Result<()> {
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

    let log = EventLog::new(db.clone());
    println!("  events     : {} stored", log.count()?);

    let last_sync = settings::read_state_time(&db, "last_mal_sync_at")?;
    println!(
        "  last sync  : {}",
        last_sync
            .map(chrono_lite)
            .unwrap_or_else(|| "never".to_string())
    );

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
