use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::Gate;
use crate::models::QuestionKind;
use crate::planner::ListEntry;
use crate::questions::{Payload, QuestionStore};
use crate::settings::{self, Settings};

use super::client::MalClient;
use super::models::{airing_status, show_type, MalEntry};
use super::pkce::SYNC_STATE_KEY;
use super::token::TokenStore;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncReport {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub total: usize,
    pub relations_failed: usize,
    pub metadata_jobs_queued: usize,
    /// Titles MAL changed; one `entry_changed` question is raised per change.
    pub renamed: Vec<String>,
    /// The plan-ready entries (`plan --from-mal` replans straight from these).
    pub entries: Vec<ListEntry>,
}

impl SyncReport {
    /// One line for `genrl mal sync` and the plan command.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "MAL list: {} new, {} updated, {} unchanged, {} removed, {} total",
            self.added, self.updated, self.unchanged, self.removed, self.total
        );
        if self.relations_failed > 0 {
            s.push_str(&format!(
                " ({} season-relation lookups failed; those stay single-season)",
                self.relations_failed
            ));
        }
        if self.metadata_jobs_queued > 0 {
            s.push_str(&format!(", {} metadata refresh jobs queued", self.metadata_jobs_queued));
        }
        s
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredEntry {
    pub(crate) status: String,
    pub(crate) watched: i64,
    pub(crate) score: Option<i64>,
    pub(crate) updated_at: Option<String>,
    pub(crate) source: String,
}

impl StoredEntry {
    pub(crate) fn changed(&self, m: &MalEntry) -> bool {
        self.status != m.entry.list_status.as_str()
            || self.watched != m.entry.watched_count
            || self.score != m.score
            || self.updated_at.as_deref() != m.updated_at.as_deref()
            || self.source != m.source
    }
}

/// Fetch, diff and store the list: `shows`, `show_names` and `mal_entries`
/// mirror the list; nothing is queued — planning from the returned entries is
/// a separate (confirmed) step.
pub async fn sync(client: &MalClient<'_>) -> Result<SyncReport> {
    let db = client.db().clone();
    let now = client.clock().now();
    let settings = client.settings();
    let log = EventLog::new(db.clone());

    let fetched = client.fetch().await?;
    let mut report = SyncReport {
        total: fetched.entries.len(),
        relations_failed: fetched.relations_failed,
        ..Default::default()
    };

    let mut fetched_ids = Vec::with_capacity(fetched.entries.len());
    for m in &fetched.entries {
        let id = m.mal_id();
        fetched_ids.push(id);

        // Shows and names first (the matching engine reads them); then the
        // diff that decides added vs updated vs unchanged.
        upsert_show_full(&db, m)?;
        crate::planner::upsert_names(&db, &m.entry)?;

        let prev_title: Option<String> = db.with(|c| {
            Ok(c.query_row(
                "SELECT title FROM shows WHERE mal_id = ?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .ok())
        })?;

        match load_mal_entry(&db, id)? {
            None => {
                save_mal_entry(&db, m)?;
                report.added += 1;
                let _ = log.info(None, Some(id), format!("MAL sync: added \"{}\"", m.entry.title));
            }
            Some(stored) => {
                if let Some(old) = prev_title {
                    if old != m.entry.title {
                        report.renamed.push(m.entry.title.clone());
                        let detail =
                            format!("MAL renamed this entry from \"{old}\" to \"{}\"", m.entry.title);
                        let expires = i64::from(settings.question_expiry_hours) * 3600;
                        if raise_entry_changed(&db, now, id, &m.entry.title, detail, Some(expires))? {
                            let _ = log.warn(
                                None,
                                Some(id),
                                format!("MAL sync: renamed \"{old}\" to \"{}\"", m.entry.title),
                            );
                        }
                    }
                }
                if stored.changed(m) {
                    save_mal_entry(&db, m)?;
                    report.updated += 1;
                } else {
                    report.unchanged += 1;
                }
            }
        }
    }

    // Entries that left the list: drop their `mal_entries` row so the table
    // stays an honest mirror, switch the show off the automatic list, but keep
    // the show and its library (the design never deletes files).
    let known: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare("SELECT mal_id FROM mal_entries")?;
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
            let _ = log.info(None, Some(id), "MAL sync: no longer on your list");
        }
    }

    // Queue metadata refresh jobs for any show missing synopsis/poster or with stale data (> 7 days)
    if let Ok(queued) = crate::metadata::queue_stale_metadata(&db, now) {
        report.metadata_jobs_queued = queued;
    }

    settings::write_state(&db, SYNC_STATE_KEY, &now.to_string())?;
    report.entries = fetched.entries.into_iter().map(|m| m.entry).collect();
    let _ = log.info(
        None,
        None,
        format!(
            "MAL sync: {} new, {} updated, {} unchanged, {} removed, {} metadata refresh queued",
            report.added, report.updated, report.unchanged, report.removed, report.metadata_jobs_queued
        ),
    );
    Ok(report)
}

/// One `entry_changed` per show per sync — the Inbox reads them oldest first,
/// so a dozen renames still surface once each, never as a wall.
pub(crate) fn raise_entry_changed(
    db: &Db,
    now: i64,
    mal_id: i64,
    title: &str,
    detail: String,
    expires_in: Option<i64>,
) -> Result<bool> {
    if QuestionStore::of_kind(db, QuestionKind::EntryChanged, now)?
        .iter()
        .any(|q| q.mal_id == Some(mal_id))
    {
        return Ok(false);
    }
    QuestionStore::ask(
        db,
        None,
        Some(mal_id),
        &Payload::EntryChanged {
            mal_id,
            title: title.to_string(),
            detail,
        },
        now,
        expires_in,
    )?;
    Ok(true)
}

pub(crate) fn load_mal_entry(db: &Db, mal_id: i64) -> Result<Option<StoredEntry>> {
    db.with(|c| {
        Ok(c.query_row(
            "SELECT list_status, watched_count, score, updated_at, source
             FROM mal_entries WHERE mal_id = ?1",
            [mal_id],
            |r| {
                Ok(StoredEntry {
                    status: r.get(0)?,
                    watched: r.get(1)?,
                    score: r.get(2)?,
                    updated_at: r.get(3)?,
                    source: r.get(4)?,
                })
            },
        )
        .ok())
    })
}

pub(crate) fn save_mal_entry(db: &Db, m: &MalEntry) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (mal_id) DO UPDATE SET
                list_status = excluded.list_status,
                watched_count = excluded.watched_count,
                score = excluded.score,
                updated_at = excluded.updated_at,
                source = excluded.source",
            rusqlite::params![
                m.mal_id(),
                m.entry.list_status.as_str(),
                m.entry.watched_count,
                m.score,
                m.updated_at,
                m.source,
            ],
        )?;
        Ok(())
    })
}

/// The sync's own show upsert, mirroring the real MAL status and type; the
/// planner's `upsert_show` never overwrites them (it only updates the fields
/// it knows).
pub(crate) fn upsert_show_full(db: &Db, m: &MalEntry) -> Result<()> {
    let e = &m.entry;
    let media = show_type(m.media_type.as_deref());
    let airing = airing_status(m.airing_status.as_deref());
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, english_title, type, status, total_episodes,
                                start_year, auto, episode_offset, season, poster_path, synopsis)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (mal_id) DO UPDATE SET
                title = excluded.title,
                english_title = COALESCE(excluded.english_title, shows.english_title),
                type = excluded.type,
                status = COALESCE(excluded.status, shows.status),
                total_episodes = COALESCE(excluded.total_episodes, shows.total_episodes),
                start_year = COALESCE(excluded.start_year, shows.start_year),
                auto = excluded.auto,
                episode_offset = excluded.episode_offset,
                season = excluded.season,
                poster_path = COALESCE(excluded.poster_path, shows.poster_path),
                synopsis = COALESCE(excluded.synopsis, shows.synopsis)",
            rusqlite::params![
                e.mal_id,
                e.title,
                e.english_title,
                media,
                airing,
                e.total_episodes,
                e.start_year,
                e.auto as i64,
                e.episode_offset,
                e.season,
                m.poster_url,
                m.synopsis,
            ],
        )?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// The cadence
// ---------------------------------------------------------------------------

/// The design's "sync at most every 30 minutes": due when the last sync is
/// older than the interval, or has never happened. Cheap — a settings read.
pub fn is_due(db: &Db, clock: &dyn Clock, interval_min: u32) -> Result<bool> {
    match settings::read_state_time(db, SYNC_STATE_KEY)? {
        Some(last) => Ok(clock.now() - last >= i64::from(interval_min) * 60),
        None => Ok(true),
    }
}

/// Sync when connected and the cadence allows it — what `genrl run` calls
/// every tick. A failure is logged, never fatal: a MAL outage must not stop
/// the workers.
pub async fn sync_if_due(
    db: &Db,
    gate: &Gate,
    clock: &dyn Clock,
    store: &dyn TokenStore,
) -> Option<SyncReport> {
    let settings = match Settings::load(db) {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!("could not load settings for MAL sync: {err}");
            return None;
        }
    };
    if settings.mal_client_id.trim().is_empty() {
        return None;
    }
    match is_due(db, clock, settings.mal_sync_interval_min) {
        Ok(true) => {}
        _ => return None,
    }
    if store.refresh_token().ok().flatten().is_none() {
        return None;
    }
    let client = MalClient::new(gate, settings, store, clock);
    match sync(&client).await {
        Ok(report) => Some(report),
        Err(err) => {
            tracing::warn!("MAL sync failed: {err}");
            let _ = EventLog::new(db.clone()).warn(None, None, format!("MAL sync failed: {err}"));
            let _ = settings::write_state_now(db, SYNC_STATE_KEY);
            None
        }
    }
}
