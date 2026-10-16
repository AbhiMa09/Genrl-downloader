use std::collections::HashMap;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::{Gate, GetOptions, Service};
use crate::settings::{self, Settings};

use super::jikan_parse::{fetch_jikan_list, fetch_mal_public_load_json, JikanAnimeDetailResponse};
use super::models::{build_chains, Node, Relation};
use super::pkce::SYNC_STATE_KEY;
use super::sync::{load_mal_entry, save_mal_entry, upsert_show_full, SyncReport};

/// Sync anime list using public Jikan profile.
pub async fn sync_jikan(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    username: &str,
) -> Result<SyncReport> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let entries_res = fetch_mal_public_load_json(gate, username).await;
    let (mut entries, is_load_json) = match entries_res {
        Ok(list) if !list.is_empty() => (list, true),
        Ok(_) => (fetch_jikan_list(gate, settings, username).await?, false),
        Err(e) => {
            tracing::warn!(
                "MAL load.json public profile fetch failed ({e}); trying Jikan fallback"
            );
            (fetch_jikan_list(gate, settings, username).await?, false)
        }
    };

    let mut report = SyncReport {
        total: entries.len(),
        ..Default::default()
    };

    let mut relations: HashMap<i64, Vec<Relation>> = HashMap::new();
    let mut relations_failed = 0usize;

    if !is_load_json {
        let raw_base = settings.jikan_base_url.trim_end_matches('/');
        let base = if raw_base.ends_with("/v4") {
            raw_base.to_string()
        } else {
            format!("{raw_base}/v4")
        };

        for m in &mut entries {
            let id = m.mal_id();
            let detail_url = format!("{base}/anime/{id}/full");
            match gate
                .get_with(Service::Jikan, &detail_url, &GetOptions::default())
                .await
            {
                Ok(fetched) => {
                    if let Ok(detail) = fetched.json::<JikanAnimeDetailResponse>() {
                        let d = detail.data;
                        m.entry.english_title = d.title_english.clone();
                        m.entry.total_episodes = d.episodes.filter(|n| *n > 0);
                        m.entry.start_year = d.year;
                        m.media_type = d.media_type.clone();
                        m.airing_status = d.status.clone();

                        let mut names = vec![d.title.clone()];
                        if let Some(en) = d.title_english {
                            if !names.contains(&en) {
                                names.push(en);
                            }
                        }
                        if let Some(ja) = d.title_japanese {
                            if !names.contains(&ja) {
                                names.push(ja);
                            }
                        }
                        if let Some(titles) = d.titles {
                            for t in titles {
                                if !names.contains(&t.title) {
                                    names.push(t.title);
                                }
                            }
                        }
                        m.entry.names = names;

                        let mut rels = Vec::new();
                        if let Some(r_list) = d.relations {
                            for r in r_list {
                                let rel_type = r.relation.to_lowercase();
                                for sub in r.entry {
                                    if sub.entry_type == "anime" {
                                        rels.push(Relation {
                                            node: Node {
                                                id: sub.mal_id,
                                                title: Some(sub.name.clone()),
                                                main_title: Some(sub.name),
                                                alternative_titles: None,
                                                num_episodes: None,
                                                start_season: None,
                                                media_type: None,
                                                status: None,
                                                main_picture: None,
                                                synopsis: None,
                                            },
                                            relation_type: rel_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        relations.insert(id, rels);
                    }
                }
                Err(_) => {
                    relations_failed += 1;
                }
            }
        }
    }

    report.relations_failed = relations_failed;
    build_chains(&mut entries, &relations);

    let mut fetched_ids = Vec::with_capacity(entries.len());
    for m in &entries {
        let id = m.mal_id();
        fetched_ids.push(id);

        upsert_show_full(db, m)?;
        crate::planner::upsert_names(db, &m.entry)?;

        let prev_title: Option<String> = db.with(|c| {
            Ok(
                c.query_row("SELECT title FROM shows WHERE mal_id = ?1", [id], |r| {
                    r.get::<_, String>(0)
                })
                .ok(),
            )
        })?;

        match load_mal_entry(db, id)? {
            None => {
                save_mal_entry(db, m)?;
                report.added += 1;
                let _ = log.info(
                    None,
                    Some(id),
                    format!("Jikan sync: added \"{}\"", m.entry.title),
                );
            }
            Some(stored) => {
                if let Some(old) = prev_title {
                    if old != m.entry.title {
                        report.renamed.push(m.entry.title.clone());
                    }
                }
                if stored.changed(m) {
                    save_mal_entry(db, m)?;
                    report.updated += 1;
                } else {
                    report.unchanged += 1;
                }
            }
        }
    }

    let known: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare("SELECT mal_id FROM mal_entries WHERE source = 'jikan'")?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        Ok(rows.filter_map(std::result::Result::ok).collect::<Vec<_>>())
    })?;
    for id in known {
        if !fetched_ids.contains(&id) {
            db.with(|c| {
                c.execute("DELETE FROM mal_entries WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            db.with(|c| {
                c.execute("UPDATE shows SET auto = 0 WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            report.removed += 1;
            let _ = log.info(None, Some(id), "Jikan sync: no longer on public list");
        }
    }

    // Queue metadata refresh jobs for any show missing synopsis/poster or with stale data (> 7 days)
    if let Ok(queued) = crate::metadata::queue_stale_metadata(db, now) {
        report.metadata_jobs_queued = queued;
    }

    settings::write_state(db, SYNC_STATE_KEY, &now.to_string())?;
    report.entries = entries.into_iter().map(|m| m.entry).collect();
    let _ = log.info(
        None,
        None,
        format!(
            "Jikan sync: {} new, {} updated, {} unchanged, {} removed, {} metadata refresh queued",
            report.added,
            report.updated,
            report.unchanged,
            report.removed,
            report.metadata_jobs_queued
        ),
    );
    Ok(report)
}
