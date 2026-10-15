use std::path::Path;
use anyhow::{Context, Result};
use genrl_core::clock::SystemClock;
use genrl_core::http::Gate;
use genrl_core::mal::{self, KeyringStore, MalClient};
use genrl_core::planner::{MalList, PlanOutcome, PlanPreview};
use genrl_core::{planner, Settings};

use super::common::open_db;

pub async fn plan_cmd(
    data_dir: &Path,
    file: Option<&Path>,
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
