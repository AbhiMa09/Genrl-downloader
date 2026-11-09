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
            .map(|s| crate::models::AiringStatus::parse(&s) == crate::models::AiringStatus::Finished)
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

    pub(crate) fn is_batch_release_or_torrent(&self, name: &str, mal_id: i64) -> bool {
        if name.to_ascii_lowercase().contains("batch")
            || crate::filename::parse_release(name).is_batch
        {
            return true;
        }
        let release = crate::filename::parse_release(name);
        if release.episode.is_some() {
            return false;
        }
        self.db
            .with(|c| {
                use rusqlite::OptionalExtension;
                let total: Option<i64> = c
                    .query_row(
                        "SELECT total_episodes FROM shows WHERE mal_id = ?1",
                        [mal_id],
                        |r| r.get(0),
                    )
                    .optional()?
                    .flatten();
                let count: i64 = c.query_row(
                    "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1",
                    [mal_id],
                    |r| r.get(0),
                )?;
                let show_type: String = c
                    .query_row(
                        "SELECT type FROM shows WHERE mal_id = ?1",
                        [mal_id],
                        |r| r.get(0),
                    )
                    .unwrap_or_else(|_| "tv".to_string());
                Ok(show_type != "movie" && (total.unwrap_or(0) > 1 || count > 1))
            })
            .unwrap_or(false)
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
            self.ask(job, mal_id, payload, &format!("unmapped season {season_no}"))?;
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
