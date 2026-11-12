use crate::error::Result;
use crate::jobs::MoveOptions;
use crate::models::{Job, JobState};
use crate::questions::Payload;
use crate::rules::Rules;
use crate::settings::Settings;

use super::engine::Engine;

impl Engine {
    pub(crate) fn is_pack_grab(&self, mal_id: i64, settings: &Settings) -> Result<bool> {
        let count: i64 = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND state = 'missing'",
                [mal_id],
                |r| r.get(0),
            )
            .unwrap_or(0))
        })?;
        let rules = Rules::for_show(&self.db, mal_id)?;
        let (status_str, total): (Option<String>, Option<i64>) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT status, total_episodes FROM shows WHERE mal_id = ?1",
                [mal_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((None, None)))
        })?;
        let is_finished = status_str
            .map(|s| {
                crate::models::AiringStatus::parse(&s) == crate::models::AiringStatus::Finished
            })
            .unwrap_or(false);
        let has_season_map = self
            .db
            .with(|c| crate::season_map::latest_season(c, mal_id))
            .ok()
            .flatten()
            .is_some();

        Ok(count >= settings.pack_first_min_missing as i64
            || (rules.prefer_batch_when_finished && is_finished && count > 1)
            || (total.unwrap_or(0) <= 0 && has_season_map && count > 1))
    }

    pub(crate) fn select_fitting_candidates(
        &self,
        ranked: Vec<crate::matching::Ranked>,
        settings: &Settings,
    ) -> (Vec<crate::matching::Ranked>, bool) {
        let download_path = crate::locations::effective_download_dir(settings);
        let floor_bytes = settings.disk_floor_gb.saturating_mul(1024 * 1024 * 1024);
        if let Some(free) = self.disk.free_bytes(&download_path) {
            let usable = free.saturating_sub(floor_bytes);
            let fitting: Vec<crate::matching::Ranked> = ranked
                .iter()
                .filter(|r| r.candidate.size_bytes <= usable)
                .cloned()
                .collect();
            let had_candidates = !ranked.is_empty();
            let none_fit = had_candidates && fitting.is_empty();
            if !fitting.is_empty() {
                return (fitting, false);
            }
            return (ranked, none_fit);
        }
        (ranked, false)
    }

    pub(crate) fn is_proven_batch_release(&self, name: &str) -> bool {
        let raw_lower = name.to_ascii_lowercase();
        if raw_lower.contains("batch")
            || raw_lower.contains("complete")
            || raw_lower.contains("compilation")
            || raw_lower.contains("all episodes")
            || raw_lower.contains("entire season")
        {
            return true;
        }
        let release = crate::filename::parse_release(name);
        if release.is_batch {
            return true;
        }
        if release.episode_range.is_some() || release.season_range.is_some() {
            return true;
        }
        if release.season.is_some() && release.episode.is_none() {
            return true;
        }
        false
    }

    pub(crate) fn handle_single_add(
        &self,
        job: &Job,
        torrent: &super::types::ChosenTorrent,
    ) -> Result<()> {
        self.db.with(|c| {
            c.execute(
                "UPDATE episodes SET state = 'downloading', torrent_id = (
                     SELECT id FROM torrents WHERE info_hash = ?1
                 )
                 WHERE id = ?2",
                rusqlite::params![torrent.info_hash, job.episode_id],
            )?;
            Ok(())
        })
    }

    pub(crate) fn handle_batch_add(
        &self,
        job: &Job,
        mal_id: i64,
        torrent: &super::types::ChosenTorrent,
    ) -> Result<()> {
        let release = crate::filename::parse_release(&torrent.name);
        let range = release.episode_range;

        self.db.with(|c| {
            if let Some((from, to)) = range {
                c.execute(
                    "UPDATE episodes SET state = 'downloading', torrent_id = (
                         SELECT id FROM torrents WHERE info_hash = ?1
                     )
                     WHERE mal_id = ?2 AND number >= ?3 AND number <= ?4
                       AND state IN ('missing', 'queued')",
                    rusqlite::params![torrent.info_hash, mal_id, from, to],
                )?;
            }

            c.execute(
                "UPDATE episodes SET state = 'downloading', torrent_id = (
                     SELECT id FROM torrents WHERE info_hash = ?1
                 )
                 WHERE id = ?2",
                rusqlite::params![torrent.info_hash, job.episode_id],
            )?;

            let sibling_jobs: Vec<i64> = match range {
                Some((from, to)) => {
                    let mut stmt = c.prepare(
                        "SELECT j.id FROM jobs j
                         JOIN episodes e ON e.id = j.episode_id
                         WHERE j.id != ?1 AND e.mal_id = ?2
                           AND e.number >= ?3 AND e.number <= ?4
                           AND j.kind = 'search'",
                    )?;
                    let rows = stmt.query_map(
                        rusqlite::params![job.id, mal_id, from, to],
                        |r| r.get(0),
                    )?;
                    rows.filter_map(|r| r.ok()).collect()
                }
                None => Vec::new(),
            };

            for sibling_id in sibling_jobs {
                let questions: Vec<i64> = {
                    let mut stmt = c.prepare(
                        "SELECT id FROM questions WHERE job_id = ?1 AND answer IS NULL",
                    )?;
                    let rows = stmt.query_map([sibling_id], |r| r.get(0))?;
                    rows.filter_map(|r| r.ok()).collect()
                };
                for q_id in questions {
                    let ans = serde_json::to_string(&crate::questions::Answer {
                        choice: crate::questions::Choice::SupersededByBatch,
                        always: false,
                    })
                    .unwrap_or_else(|_| "{\"choice\":{\"choice\":\"superseded_by_batch\"},\"always\":false}".to_string());
                    c.execute(
                        "UPDATE questions SET answer = ?1, expires_at = ?2, job_id = NULL WHERE id = ?3",
                        rusqlite::params![ans, self.now(), q_id],
                    )?;
                    crate::events::record_in(
                        c,
                        crate::events::Level::Info,
                        Some(sibling_id),
                        Some(mal_id),
                        &format!("question #{q_id} closed: sibling job superseded by batch add"),
                        self.now(),
                    )?;
                }
                c.execute("DELETE FROM jobs WHERE id = ?1", [sibling_id])?;
            }

            Ok(())
        })
    }

    pub(crate) fn try_ask_unmapped_season(
        &self,
        job: &Job,
        mal_id: i64,
        rejections: &[(String, crate::score::Rejection)],
    ) -> Result<Option<JobState>> {
        if let Some((raw_title, season_no)) = rejections.iter().find_map(|(raw, rej)| {
            if let crate::score::Rejection::SeasonConflict { found, wanted } = rej {
                if *found != *wanted && *found > 0 {
                    return Some((raw, *found as u32));
                }
            }
            None
        }) {
            let parsed = crate::filename::parse_release(raw_title);
            let ep_count = parsed
                .episode_range
                .map(|(f, t)| (t - f + 1.0) as u32)
                .unwrap_or(0);
            let (suggested_first, suggested_last) = self
                .db
                .with(|c| crate::season_map::suggest_range(c, mal_id, ep_count))?;
            let payload = Payload::SeasonMap {
                mal_id,
                season_no,
                release_title: raw_title.clone(),
                suggested_first,
                suggested_last,
                reason: format!(
                    "Release specifies Season {season_no}, which is not mapped to episode numbers for this show"
                ),
            };
            self.ask(
                job,
                mal_id,
                payload,
                &format!("unmapped season {season_no}"),
            )?;
            let next = crate::jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::NeedsInput,
                &format!("unmapped season {season_no}"),
                &MoveOptions::default(),
            )?;
            return Ok(Some(next));
        }
        Ok(None)
    }
}
