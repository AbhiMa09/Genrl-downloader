use anyhow::Result;
use clap::Subcommand;
use genrl_core::http::Gate;
use genrl_core::Settings;
use std::path::Path;

use super::common::open_db;

#[derive(Subcommand, Debug)]
pub enum MetadataCommand {
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

pub async fn metadata_fetch_cmd(
    data_dir: &Path,
    mal_id: i64,
    force: bool,
    json: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let gate = Gate::new(db.clone())?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    let outcome =
        genrl_core::fetch_show_metadata(&db, &gate, &settings, &clock, data_dir, mal_id, force)
            .await?;

    if json {
        let val = match outcome {
            genrl_core::MetadataOutcome::Updated {
                mal_id,
                synopsis,
                poster_path,
            } => {
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
            genrl_core::MetadataOutcome::PendingRetry {
                mal_id,
                next_run_at,
                error,
            } => {
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
            genrl_core::MetadataOutcome::PendingRetry {
                mal_id,
                next_run_at,
                error,
            } => {
                println!("Metadata fetch failed for show #{mal_id}: {error}. Next retry at {next_run_at}");
            }
            genrl_core::MetadataOutcome::Failed { mal_id, error } => {
                println!("Metadata fetch failed for show #{mal_id}: {error}");
            }
        }
    }
    Ok(())
}

pub async fn metadata_sync_cmd(data_dir: &Path, limit: usize, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let gate = Gate::new(db.clone())?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    let report =
        genrl_core::process_metadata_queue(&db, &gate, &settings, &clock, data_dir, Some(limit))
            .await?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "processed": report.processed,
                "updated": report.updated,
                "failed": report.failed,
                "cached": report.cached,
            }))?
        );
    } else {
        println!(
            "Metadata queue: {} processed, {} updated, {} cached, {} pending retry",
            report.processed, report.updated, report.cached, report.failed
        );
    }
    Ok(())
}
