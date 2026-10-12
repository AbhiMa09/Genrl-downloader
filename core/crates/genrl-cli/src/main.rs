//! `genrl` — command-line companion to the Genrl Downloader core.
//!
//! Phase 1 gives it init, settings, activity and a `probe` that walks a request
//! through the whole limit-proof gate. Later phases extend it with the
//! end-to-end checks from the roadmap.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use genrl_core::http::{Gate, Service};
use genrl_core::{settings, Db, EventLog, Settings};

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

#[cfg(test)]
mod tests {
    use super::chrono_lite;

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(chrono_lite(0), "1970-01-01 00:00:00");
        assert_eq!(chrono_lite(1_760_000_000), "2025-10-09 08:53:20");
    }
}
