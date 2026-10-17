use anyhow::{Context, Result};
use genrl_core::clock::SystemClock;
use genrl_core::http::Gate;
use genrl_core::mal::{self, KeyringStore, MalClient};
use genrl_core::planner::{MalList, PlanOutcome, PlanPreview};
use genrl_core::{planner, Settings};
use std::path::Path;

use super::common::open_db;

#[derive(Default)]
pub struct PlanOptions<'a> {
    pub file: Option<&'a Path>,
    pub dry_run: bool,
    pub confirm: bool,
    pub from_mal: bool,
    pub mal_id: Option<i64>,
    pub episode: Option<f64>,
    pub bypass_cap: bool,
    pub json: bool,
}

pub async fn plan_cmd(data_dir: &Path, opts: PlanOptions<'_>) -> Result<()> {
    let db = open_db(data_dir)?;
    let clock = SystemClock;

    let PlanOptions {
        file,
        dry_run,
        confirm,
        from_mal,
        mal_id,
        episode,
        bypass_cap,
        json,
    } = opts;

    // Targeted single show or episode download
    if let Some(target_id) = mal_id {
        if dry_run {
            if json {
                println!(
                    r#"{{"dry_run":true,"mal_id":{target_id},"episode":{:?}}}"#,
                    episode
                );
                return Ok(());
            }
            println!(
                "dry run: would queue show #{target_id} (episode: {:?})",
                episode
            );
            return Ok(());
        }
        let queued = planner::plan_single(&db, &clock, target_id, episode)?;
        let mut settings = Settings::load(&db)?;
        settings.first_run_complete = true;
        settings.save(&db)?;
        if json {
            println!(r#"{{"queued":{queued},"mal_id":{target_id}}}"#);
            return Ok(());
        }
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
                if !json {
                    println!("{}", report.summary());
                }
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
        let preview = planner::preview(&db, &list)?;
        if json {
            println!("{}", serde_json::to_string_pretty(&preview)?);
            return Ok(());
        }
        print_preview(&preview);
        println!("dry run: nothing was queued");
        return Ok(());
    }

    match planner::plan(&db, &clock, &list, true)? {
        PlanOutcome::Applied { queued, preview } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&preview)?);
                return Ok(());
            }
            print_preview(&preview);
            println!(
                "queued {queued} new job{}",
                if queued == 1 { "" } else { "s" }
            );
        }
        PlanOutcome::Preview(preview) => {
            if !confirm {
                if json {
                    println!("{}", serde_json::to_string_pretty(&preview)?);
                    return Ok(());
                }
                print_preview(&preview);
                println!("the first sync awaits one confirmation; re-run with --confirm");
                return Ok(());
            }
            let queued = planner::confirm_capped(&db, &clock, &list, bypass_cap)?;
            let mut settings = Settings::load(&db)?;
            settings.first_run_complete = true;
            settings.save(&db)?;
            if json {
                println!(r#"{{"queued":{queued},"first_sync_complete":true}}"#);
                return Ok(());
            }
            print_preview(&preview);
            println!(
                "confirmed: queued {queued} new job{}; first sync complete",
                if queued == 1 { "" } else { "s" }
            );
        }
    }
    Ok(())
}

/// One preview, three callers: dry run, first sync, applied plan.
pub fn print_preview(preview: &PlanPreview) {
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
