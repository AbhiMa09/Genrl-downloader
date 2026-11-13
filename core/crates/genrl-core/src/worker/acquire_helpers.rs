use super::types::ChosenTorrent;
use crate::error::{Error, Result};
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
        let Some(ep_id) = job.episode_id else {
            return Ok(());
        };
        self.db.with(|c| {
            c.execute(
                "UPDATE episodes SET state = 'downloading', torrent_id = (
                     SELECT id FROM torrents WHERE info_hash = ?1
                 )
                 WHERE id = ?2",
                rusqlite::params![torrent.info_hash, ep_id],
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
        let Some(ep_id) = job.episode_id else {
            return Ok(());
        };
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
                rusqlite::params![torrent.info_hash, ep_id],
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

    pub(crate) async fn add_torrent_to_qbit(
        &self,
        job: &Job,
        torrent: &ChosenTorrent,
        save_path: &str,
        category: &str,
        settings: &Settings,
    ) -> Result<()> {
        let client = crate::qbit::Qbit::from_settings(settings)?;
        let tag_kind = job.media_kind.as_deref().unwrap_or("anime");
        let tags = ["genrl", tag_kind];

        let is_torrent_url =
            torrent.magnet.starts_with("http://") || torrent.magnet.starts_with("https://");
        if is_torrent_url {
            let breaker_key = reqwest::Url::parse(&torrent.magnet)
                .ok()
                .and_then(|u| u.host_str().map(|h| format!("indexer:{h}")))
                .unwrap_or_else(|| "indexer:default".to_string());
            let fetched = match self.gate.get_keyed(&breaker_key, &torrent.magnet).await {
                Ok(f) => f,
                Err(err) if err.is_transient() || err.is_connection_failure() => {
                    return Err(Error::Other(format!("indexer download unreachable: {err}")));
                }
                Err(err) => {
                    return Err(Error::Other(format!("failed downloading .torrent: {err}")))
                }
            };

            if fetched.body.len() > crate::torrent::bencode::MAX_TORRENT_SIZE {
                return Err(Error::Other("torrent file exceeds 10 MB cap".into()));
            }

            let torrent_info = crate::torrent::bencode::parse_torrent_info_hash(&fetched.body)?;
            let computed_hash = &torrent_info.info_hash;

            let redacted_url = crate::indexer::redact_secrets(&torrent.magnet);
            self.db.with(|c| {
                c.execute(
                    "UPDATE torrents SET info_hash = ?1, magnet = ?2 WHERE info_hash = ?3 OR magnet = ?4",
                    rusqlite::params![computed_hash, redacted_url, torrent.info_hash, torrent.magnet],
                )?;
                Ok(())
            })?;

            let sanitized = crate::library::layout::sanitize_component(&torrent.name)
                .unwrap_or_else(|_| "download".to_string());
            let filename = format!("{sanitized}.torrent");
            client
                .add_torrent_file(
                    &self.gate,
                    &fetched.body,
                    &filename,
                    Some(save_path),
                    Some(category),
                    &tags,
                )
                .await?;
        } else {
            client
                .add_with_tags(
                    &self.gate,
                    std::slice::from_ref(&torrent.magnet),
                    Some(save_path),
                    Some(category),
                    &tags,
                )
                .await?;
        }
        Ok(())
    }

    pub(crate) fn job_search_queries(
        &self,
        job: &Job,
        context: &super::types::JobContext,
        settings: &Settings,
    ) -> Result<(crate::matching::TargetInputs, Vec<String>)> {
        let is_movie = job.media_kind.as_deref() == Some("movie");
        if is_movie {
            let (title, year) = if let Some(media_id) = job.media_id {
                self.db.with(|c| {
                    Ok(c.query_row(
                        "SELECT title, year FROM media WHERE id = ?1",
                        [media_id],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)),
                    )?)
                })?
            } else {
                (context.title.clone(), None)
            };
            Ok((
                crate::matching::TargetInputs::for_movie(&title, year),
                vec![title],
            ))
        } else {
            let inputs = crate::matching::TargetInputs::load(&self.db, context.mal_id)?;
            let rules = Rules::for_show(&self.db, context.mal_id)?;
            let is_pack_grab = self.is_pack_grab(context.mal_id, settings)?;
            let mut search_names = inputs.names.clone();
            if is_pack_grab {
                let mut pack_queries = Vec::new();
                for n in &inputs.names {
                    pack_queries.push(n.clone());
                    pack_queries.push(format!("{n} batch"));
                    pack_queries.push(format!("{n} complete"));
                }
                search_names = pack_queries;
            } else if inputs.fully_aired && rules.prefer_batch_when_finished {
                let mut batch_queries = Vec::new();
                for n in &inputs.names {
                    batch_queries.push(format!("{n} Batch"));
                    batch_queries.push(format!("{n} Season {}", inputs.season));
                }
                batch_queries.extend(search_names);
                search_names = batch_queries;
            } else {
                let ep_int = context.number as i64;
                let mut ep_queries = Vec::new();
                for n in &inputs.names {
                    ep_queries.push(format!("{n} - {ep_int}"));
                    ep_queries.push(format!("{n} {ep_int}"));
                    ep_queries.push(format!("{n} - {:02}", ep_int));
                    ep_queries.push(format!("{n} {:02}", ep_int));
                }
                search_names = ep_queries;
            }
            Ok((inputs, search_names))
        }
    }

    pub(crate) fn choose_inputs(
        &self,
        job: &Job,
        context: &super::types::JobContext,
    ) -> Result<(
        crate::matching::TargetInputs,
        Vec<crate::score::Candidate>,
        Rules,
    )> {
        let is_movie = job.media_kind.as_deref() == Some("movie");
        if is_movie {
            let (title, year) = if let Some(media_id) = job.media_id {
                self.db.with(|c| {
                    Ok(c.query_row(
                        "SELECT title, year FROM media WHERE id = ?1",
                        [media_id],
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)),
                    )?)
                })?
            } else {
                (context.title.clone(), None)
            };
            Ok((
                crate::matching::TargetInputs::for_movie(&title, year),
                self.candidate_rows(0)?,
                Rules::default(),
            ))
        } else {
            Ok((
                crate::matching::TargetInputs::load(&self.db, context.mal_id)?,
                self.candidate_rows(context.mal_id)?,
                Rules::for_show(&self.db, context.mal_id)?,
            ))
        }
    }
}
