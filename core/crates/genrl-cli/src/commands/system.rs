use anyhow::{bail, Context, Result};
use clap::Subcommand;
use genrl_core::http::{Gate, Service};
use genrl_core::{settings, Db, EventLog, Settings};
use std::path::{Path, PathBuf};

use super::common::{chrono_lite, open_db};

#[derive(Subcommand, Debug)]
pub enum SettingsCommand {
    /// Print the current settings as JSON.
    Show,
    /// Export settings to a JSON file.
    Export { path: PathBuf },
    /// Import settings from a JSON file (validates first).
    Import { path: PathBuf },
}

pub fn init(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    println!("data dir : {}", data_dir.display());
    println!("database : {}", db.path().unwrap().display());
    println!("schema   : v{}", db.schema_version()?);
    println!("settings : {} keys stored", count_settings(&db)?);
    println!();
    println!("Next: genrl settings show | genrl doctor");
    Ok(())
}

pub fn count_settings(db: &Db) -> Result<i64> {
    Ok(db.with(|c| {
        let n: i64 = c.query_row(
            "SELECT COUNT(*) FROM settings WHERE key NOT LIKE 'state.%'",
            [],
            |r| r.get(0),
        )?;
        Ok(n)
    })?)
}

pub fn settings_cmd(data_dir: &Path, command: SettingsCommand) -> Result<()> {
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

pub fn events(data_dir: &Path, limit: u32, errors_only: bool, json: bool) -> Result<()> {
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

pub async fn probe(data_dir: &Path, service: &str, url: &str, skip_cache: bool) -> Result<()> {
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

pub fn doctor(data_dir: &Path) -> Result<()> {
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
        } else if Path::new(folder).is_dir() {
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

    println!(
        "  automation  : {}",
        if settings.automation_on { "ON" } else { "off" }
    );
    Ok(())
}
