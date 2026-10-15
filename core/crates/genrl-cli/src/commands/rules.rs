use std::path::Path;
use anyhow::Result;
use clap::Subcommand;
use genrl_core::EventLog;

use super::common::open_db;

#[derive(Subcommand, Debug)]
pub enum RulesCommand {
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

pub fn rules_get_cmd(data_dir: &Path, mal_id: Option<i64>) -> Result<()> {
    let db = open_db(data_dir)?;
    let r = genrl_core::rules::get_rules(&db, mal_id)?;
    println!("{}", serde_json::to_string_pretty(&r)?);
    Ok(())
}

pub fn rules_set_cmd(
    data_dir: &Path,
    key: &str,
    val_str: &str,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    let value: serde_json::Value = serde_json::from_str(val_str)
        .unwrap_or_else(|_| serde_json::Value::String(val_str.to_string()));
    genrl_core::rules::update_rule(&db, mal_id, key, value, &log)?;
    println!(
        "rule updated: {key} for {}",
        if let Some(id) = mal_id {
            format!("show #{id}")
        } else {
            "global".into()
        }
    );
    Ok(())
}

pub fn rules_reset_cmd(
    data_dir: &Path,
    key: Option<&str>,
    mal_id: Option<i64>,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let log = EventLog::new(db.clone());
    genrl_core::rules::reset_rule(&db, mal_id, key, &log)?;
    println!(
        "rule(s) reset for {}",
        if let Some(id) = mal_id {
            format!("show #{id}")
        } else {
            "global".into()
        }
    );
    Ok(())
}
