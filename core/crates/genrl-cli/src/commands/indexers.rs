use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Result};
use clap::Subcommand;
use serde::{Deserialize, Serialize};

use genrl_core::http::Gate;
use genrl_core::indexer::config::{delete_api_key, validate_indexer_url, IndexerConfig};
use genrl_core::indexer::fetch::GateFetch;
use genrl_core::indexer::redact::redact_secrets;
use genrl_core::indexer::torznab_caps::{delete_cached_caps, parse_caps};
use genrl_core::indexer::types::MediaKind;

use super::common::open_db;

#[derive(Subcommand, Debug, Clone)]
pub enum IndexersCommand {
    /// List all configured Torznab indexers without secrets.
    List {
        #[arg(long)]
        json: bool,
    },
    /// Add a new Torznab indexer.
    Add {
        id: String,
        name: String,
        url: String,
        /// Media kinds: movie, series (comma-separated, default: movie,series).
        #[arg(long, value_delimiter = ',')]
        kinds: Option<Vec<String>>,
        /// Read API key from this environment variable name.
        #[arg(long)]
        key_env: Option<String>,
        /// Test connectivity (t=caps) immediately upon adding.
        #[arg(long)]
        test: bool,
        #[arg(long)]
        json: bool,
    },
    /// Remove an indexer, its credentials, cached caps, and circuit breaker state.
    Remove {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Test an indexer by executing a live t=caps query through Gate.
    Test {
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Enable or disable an indexer in search dispatch.
    SetEnabled {
        id: String,
        #[arg(action = clap::ArgAction::Set, value_parser = clap::value_parser!(bool))]
        enabled: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexerSummary {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub enabled: bool,
    pub kinds: Vec<MediaKind>,
    pub has_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_test_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_test_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IndexerTestResult {
    pub ok: bool,
    pub categories_count: usize,
    pub movie_search: bool,
    pub tv_search: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn check_forbidden_key_args() -> Result<()> {
    for arg in std::env::args() {
        if arg == "--apikey"
            || arg.starts_with("--apikey=")
            || arg == "--key"
            || arg.starts_with("--key=")
        {
            bail!("API keys must never be passed as command-line arguments (visible in process list). Pass the key via stdin or use --key-env <VAR_NAME>.");
        }
    }
    Ok(())
}

pub fn read_api_key_from_stdin_or_env(key_env: Option<&str>) -> Result<String> {
    if let Some(env_name) = key_env {
        let val = std::env::var(env_name).map_err(|_| {
            anyhow::anyhow!("Environment variable '{env_name}' is not set or empty")
        })?;
        let trimmed = val.trim().to_string();
        if trimmed.is_empty() {
            bail!("Environment variable '{env_name}' contains an empty API key");
        }
        return Ok(trimmed);
    }

    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut line = String::new();
    let bytes_read = stdin.lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();
    if bytes_read == 0 || trimmed.is_empty() {
        bail!("No API key provided on stdin. Provide a single line via stdin or pass --key-env <NAME>.");
    }
    Ok(trimmed)
}

pub async fn indexers_cmd(data_dir: &Path, command: IndexersCommand) -> Result<()> {
    check_forbidden_key_args()?;
    let db = open_db(data_dir)?;

    match command {
        IndexersCommand::List { json } => {
            let configs = IndexerConfig::load_all(&db)?;
            let summaries: Vec<IndexerSummary> = configs
                .into_iter()
                .map(|c| IndexerSummary {
                    id: c.id,
                    name: c.name,
                    base_url: c.base_url,
                    enabled: c.enabled,
                    kinds: c.media_kinds,
                    has_key: c.api_key.is_some(),
                    last_test_status: c.last_test_status,
                    last_test_at: c.last_test_at,
                })
                .collect();

            if json {
                println!("{}", serde_json::to_string_pretty(&summaries)?);
            } else if summaries.is_empty() {
                println!("No indexers configured. Add one with `genrl indexers add`.");
            } else {
                for s in summaries {
                    println!(
                        "{} ({}) - {} [{}] (key: {}, last test: {})",
                        s.name,
                        s.id,
                        s.base_url,
                        if s.enabled { "enabled" } else { "disabled" },
                        if s.has_key { "yes" } else { "no" },
                        s.last_test_status.as_deref().unwrap_or("never")
                    );
                }
            }
        }
        IndexersCommand::Add {
            id,
            name,
            url,
            kinds,
            key_env,
            test,
            json,
        } => {
            // 1. Validate URL first (must not store key or config if URL is invalid)
            let clean_url = validate_indexer_url(&url)?;

            // 2. Read key securely via stdin or --key-env
            let key = read_api_key_from_stdin_or_env(key_env.as_deref())?;

            // 3. Media kinds parse
            let media_kinds: Vec<MediaKind> = kinds
                .unwrap_or_else(|| vec!["movie".into(), "series".into()])
                .into_iter()
                .filter_map(|s| match s.trim().to_ascii_lowercase().as_str() {
                    "movie" | "movies" => Some(MediaKind::Movie),
                    "series" | "tv" => Some(MediaKind::Series),
                    "anime" => Some(MediaKind::Anime),
                    _ => None,
                })
                .collect();

            let mut cfg = IndexerConfig::new(&id, &name, &clean_url, Some(key), media_kinds);

            // Invalidate any previous caps cache
            let _ = delete_cached_caps(&db, &id);

            // 4. Optional test
            if test {
                let gate = Arc::new(Gate::new(db.clone())?);
                let test_res = run_indexer_test(&gate, &db, &cfg).await;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                cfg.last_test_at = Some(now);
                if test_res.ok {
                    cfg.last_test_status = Some("ok".into());
                } else {
                    cfg.last_test_status = Some(format!(
                        "failed: {}",
                        test_res.error.as_deref().unwrap_or("unknown")
                    ));
                }
            }

            // Save config and store key
            IndexerConfig::save(&db, &cfg)?;

            let summary = IndexerSummary {
                id: cfg.id,
                name: cfg.name,
                base_url: cfg.base_url,
                enabled: cfg.enabled,
                kinds: cfg.media_kinds,
                has_key: cfg.api_key.is_some(),
                last_test_status: cfg.last_test_status,
                last_test_at: cfg.last_test_at,
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!("Indexer added: {} ({})", summary.name, summary.id);
            }
        }
        IndexersCommand::Remove { id, json } => {
            // Remove config from DB
            IndexerConfig::remove(&db, &id)?;

            // Delete key from keyring / secrets file
            delete_api_key(&id);

            // Delete cached caps
            let _ = delete_cached_caps(&db, &id);

            // Remove dynamic breaker state
            if let Ok(gate) = Gate::new(db.clone()) {
                gate.remove_dynamic_state(&format!("torznab:{id}"));
                gate.remove_dynamic_state(&format!("indexer:{id}"));
            }

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "id": id,
                    }))?
                );
            } else {
                println!("Indexer removed: {id}");
            }
        }
        IndexersCommand::Test { id, json } => {
            let configs = IndexerConfig::load_all(&db)?;
            let mut cfg = configs
                .into_iter()
                .find(|c| c.id == id)
                .ok_or_else(|| anyhow::anyhow!("Indexer '{id}' not found"))?;

            let gate = Arc::new(Gate::new(db.clone())?);
            let test_res = run_indexer_test(&gate, &db, &cfg).await;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            cfg.last_test_at = Some(now);
            if test_res.ok {
                cfg.last_test_status = Some("ok".into());
            } else {
                cfg.last_test_status = Some(format!(
                    "failed: {}",
                    test_res.error.as_deref().unwrap_or("unknown")
                ));
            }
            let _ = IndexerConfig::save(&db, &cfg);

            if json {
                println!("{}", serde_json::to_string_pretty(&test_res)?);
            } else if test_res.ok {
                println!(
                    "Indexer '{}' OK: {} categories, movie_search={}, tv_search={}",
                    id, test_res.categories_count, test_res.movie_search, test_res.tv_search
                );
            } else {
                println!(
                    "Indexer '{}' FAILED: {}",
                    id,
                    test_res.error.as_deref().unwrap_or("unknown error")
                );
            }
        }
        IndexersCommand::SetEnabled { id, enabled, json } => {
            let configs = IndexerConfig::load_all(&db)?;
            let mut cfg = configs
                .into_iter()
                .find(|c| c.id == id)
                .ok_or_else(|| anyhow::anyhow!("Indexer '{id}' not found"))?;

            cfg.enabled = enabled;
            IndexerConfig::save(&db, &cfg)?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "ok": true,
                        "id": id,
                        "enabled": enabled,
                    }))?
                );
            } else {
                println!("Indexer '{id}' enabled={enabled}");
            }
        }
    }

    Ok(())
}

async fn run_indexer_test(
    gate: &Arc<Gate>,
    db: &genrl_core::Db,
    cfg: &IndexerConfig,
) -> IndexerTestResult {
    let fetch = Arc::new(GateFetch::new(Arc::clone(gate)));
    let breaker_key = format!("torznab:{}", cfg.id);
    let key = cfg.api_key.as_deref().unwrap_or("");
    let params = [("t", "caps"), ("apikey", key)];

    use genrl_core::indexer::fetch::HttpFetch;
    match fetch.get_text(&breaker_key, &cfg.base_url, &params).await {
        Ok(body) => match parse_caps(&body) {
            Ok(caps) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                let _ =
                    genrl_core::indexer::torznab_caps::save_cached_caps(db, &cfg.id, &body, now);
                IndexerTestResult {
                    ok: true,
                    categories_count: caps.categories.len(),
                    movie_search: caps.movie_available,
                    tv_search: caps.tv_available,
                    error: None,
                }
            }
            Err(e) => IndexerTestResult {
                ok: false,
                categories_count: 0,
                movie_search: false,
                tv_search: false,
                error: Some(redact_secrets(&e.to_string())),
            },
        },
        Err(e) => IndexerTestResult {
            ok: false,
            categories_count: 0,
            movie_search: false,
            tv_search: false,
            error: Some(redact_secrets(&e.to_string())),
        },
    }
}
