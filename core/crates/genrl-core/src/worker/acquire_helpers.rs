use super::{engine::Engine, types::ChosenTorrent};
use crate::error::{Error, Result};
use crate::jobs::MoveOptions;
use crate::models::{Job, JobState};
use crate::questions::Payload;
use crate::rules::Rules;
use crate::settings::Settings;

impl Engine {
    /// Check if the job's episode is already downloaded on disk (DB or Library folder).
    pub(crate) fn already_downloaded_on_disk(
        &self,
        job: &Job,
        context: &super::types::JobContext,
        settings: &Settings,
    ) -> Result<Option<String>> {
        let Some(ep_id) = job.episode_id else {
            return Ok(None);
        };
        // 1. Check DB state + file_path on disk
        let existing: Option<(String, Option<String>)> = self.db.with(|c| {
            use rusqlite::OptionalExtension;
            Ok(c.query_row(
                "SELECT state, file_path FROM episodes WHERE id = ?1",
                [ep_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
            )
            .optional()?)
        })?;
        if let Some((state, Some(path))) = existing {
            if state == "done" && std::path::Path::new(&path).is_file() {
                return Ok(Some(path));
            }
        }
        // 2. Scan the show's library folder for a matching episode file
        if job.media_kind.as_deref().unwrap_or("anime") == "anime" && context.mal_id > 0 {
            let lib_root = if settings.library_folder.trim().is_empty() {
                crate::locations::default_library_dir()
            } else {
                crate::locations::resolve_home_dir(&settings.library_folder)
            };
            let year: Option<i64> = self.db.with(|c| {
                use rusqlite::OptionalExtension;
                Ok(c.query_row(
                    "SELECT start_year FROM shows WHERE mal_id = ?1",
                    [context.mal_id],
                    |r| r.get(0),
                )
                .optional()?
                .flatten())
            })?;
            if let Ok(show_dir) = crate::library::resolve_show_folder(
                &self.db,
                &lib_root,
                context.mal_id,
                &context.title,
                year,
            ) {
                if show_dir.is_dir() {
                    let mut stack = vec![show_dir];
                    while let Some(dir) = stack.pop() {
                        let Ok(entries) = std::fs::read_dir(&dir) else {
                            continue;
                        };
                        for entry in entries.flatten() {
                            let p = entry.path();
                            if p.is_dir() {
                                stack.push(p);
                                continue;
                            }
                            let Some(ext) = p.extension().and_then(|e| e.to_str()) else {
                                continue;
                            };
                            if !matches!(
                                ext.to_ascii_lowercase().as_str(),
                                "mkv" | "mp4" | "avi" | "webm"
                            ) {
                                continue;
                            }
                            let Ok(meta) = p.metadata() else { continue };
                            if meta.len() == 0 {
                                continue;
                            }
                            let fname = p.file_name().and_then(|f| f.to_str()).unwrap_or("");
                            let parsed = crate::filename::parse_release(fname);
                            let matched = match (parsed.episode, parsed.episode_range) {
                                (Some(ep), _) => (ep - context.number).abs() < 0.01,
                                (None, Some((from, to))) => {
                                    from <= context.number && context.number <= to
                                }
                                (None, None) => false,
                            };
                            if matched {
                                let p_str = p.display().to_string();
                                let _ = crate::planner::mark_done(
                                    &self.db,
                                    ep_id,
                                    &p_str,
                                    meta.len() as i64,
                                );
                                return Ok(Some(p_str));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn is_pack_grab(&self, mal_id: i64, settings: &Settings) -> Result<bool> {
        let count: i64 = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND state = 'missing'",
                [mal_id],
                |r| r.get(0),
            )?)
        })?;
        let rules = Rules::for_show(&self.db, mal_id)?;
        let (status_str, total): (Option<String>, Option<i64>) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT status, total_episodes FROM shows WHERE mal_id = ?1",
                [mal_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?)
        })?;
        let is_finished = status_str
            .as_deref()
            .map(crate::models::AiringStatus::parse)
            == Some(crate::models::AiringStatus::Finished);
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
            if !fitting.is_empty() {
                return (fitting, false);
            }
            return (ranked, had_candidates);
        }
        (ranked, false)
    }

    pub(crate) fn is_proven_batch_release(&self, name: &str) -> bool {
        let raw = name.to_ascii_lowercase();
        if [
            "batch",
            "complete",
            "compilation",
            "all episodes",
            "entire season",
        ]
        .iter()
        .any(|w| raw.contains(w))
        {
            return true;
        }
        let rel = crate::filename::parse_release(name);
        rel.is_batch
            || rel.episode_range.is_some()
            || rel.season_range.is_some()
            || (rel.season.is_some() && rel.episode.is_none())
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
        let tags: Vec<&str> = match job.media_kind.as_deref() {
            Some("movie") => vec!["genrl", "genrl:movie"],
            Some("series") => vec!["genrl", "genrl:series"],
            _ => vec![],
        };

        let is_torrent_url =
            torrent.magnet.starts_with("http://") || torrent.magnet.starts_with("https://");
        if is_torrent_url {
            let mut download_url = torrent.magnet.clone();
            if download_url.contains("[REDACTED]") || download_url.contains("%5BREDACTED%5D") {
                if let Ok(configs) = crate::indexer::config::IndexerConfig::load_all(&self.db) {
                    for indexer in configs {
                        if let Ok(idx_url) = reqwest::Url::parse(&indexer.base_url) {
                            if let Ok(dl_url) = reqwest::Url::parse(&download_url) {
                                if idx_url.host_str() == dl_url.host_str()
                                    && idx_url.port() == dl_url.port()
                                {
                                    let idx_base = idx_url.path().trim_end_matches("/api");
                                    if dl_url.path().starts_with(idx_base) {
                                        if let Some(key) =
                                            crate::indexer::config::load_api_key(&indexer.id)
                                        {
                                            download_url = download_url
                                                .replace("[REDACTED]", &key)
                                                .replace("%5BREDACTED%5D", &key);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if let Ok(cp) = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(10))
                .build()
            {
                if let Ok(resp) = cp.get(&download_url).send().await {
                    if resp.status().is_redirection() {
                        if let Some(loc) = resp.headers().get(reqwest::header::LOCATION) {
                            if let Ok(loc_str) = loc.to_str() {
                                if loc_str.starts_with("magnet:") {
                                    return client
                                        .add_with_tags(
                                            &self.gate,
                                            &[loc_str.to_string()],
                                            Some(save_path),
                                            Some(category),
                                            &tags,
                                        )
                                        .await;
                                }
                            }
                        }
                    }
                }
            }
            let breaker_key = reqwest::Url::parse(&download_url)
                .ok()
                .and_then(|u| u.host_str().map(|h| format!("indexer:{h}")))
                .unwrap_or_else(|| "indexer:default".to_string());
            let fetched = match self.gate.get_keyed(&breaker_key, &download_url).await {
                Ok(f) => f,
                Err(err) => {
                    let redacted_url = crate::indexer::redact_secrets(&torrent.magnet);
                    let _ = self.db.with(|c| {
                        c.execute(
                            "UPDATE torrents SET magnet = ?1 WHERE magnet = ?2 OR info_hash = ?3",
                            rusqlite::params![redacted_url, torrent.magnet, torrent.info_hash],
                        )?;
                        Ok(())
                    });
                    let msg = crate::indexer::redact_secrets(&err.to_string());
                    let pfx = if err.is_transient() || err.is_connection_failure() {
                        "indexer download unreachable"
                    } else {
                        "failed downloading .torrent"
                    };
                    return Err(Error::Other(format!("{pfx}: {msg}")));
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
        let is_series = job.media_kind.as_deref() == Some("series");
        if is_movie || is_series {
            let (title, year) = match job.media_id {
                Some(id) => self.db.with(|c| {
                    Ok(
                        c.query_row("SELECT title, year FROM media WHERE id = ?1", [id], |r| {
                            Ok((r.get(0)?, r.get(1)?))
                        })?,
                    )
                })?,
                None => (context.title.clone(), None),
            };
            if is_series {
                let s = job.season.unwrap_or(1);
                let queries = match job.episode {
                    Some(ep) => vec![
                        format!("{title} S{s:02}E{ep:02}"),
                        format!("{title} S{s}E{ep}"),
                    ],
                    None => vec![
                        format!("{title} Season {s} Complete"),
                        format!("{title} Batch"),
                    ],
                };
                return Ok((
                    crate::matching::TargetInputs::for_series(&title, year),
                    queries,
                ));
            }
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
                search_names = inputs
                    .names
                    .iter()
                    .flat_map(|n| [n.clone(), format!("{n} batch"), format!("{n} complete")])
                    .collect();
            } else if inputs.fully_aired && rules.prefer_batch_when_finished {
                let mut batch_queries: Vec<String> = inputs
                    .names
                    .iter()
                    .flat_map(|n| {
                        [
                            format!("{n} Batch"),
                            format!("{n} Season {}", inputs.season),
                        ]
                    })
                    .collect();
                batch_queries.extend(search_names);
                search_names = batch_queries;
            } else {
                let ep = context.number as i64;
                search_names = inputs
                    .names
                    .iter()
                    .flat_map(|n| {
                        [
                            format!("{n} - {ep}"),
                            format!("{n} {ep}"),
                            format!("{n} - {:02}", ep),
                            format!("{n} {:02}", ep),
                        ]
                    })
                    .collect();
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
        let is_series = job.media_kind.as_deref() == Some("series");
        if is_movie || is_series {
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
            let inputs = if is_series {
                crate::matching::TargetInputs::for_series(&title, year)
            } else {
                crate::matching::TargetInputs::for_movie(&title, year)
            };
            Ok((inputs, self.candidate_rows(0)?, Rules::default()))
        } else {
            Ok((
                crate::matching::TargetInputs::load(&self.db, context.mal_id)?,
                self.candidate_rows(context.mal_id)?,
                Rules::for_show(&self.db, context.mal_id)?,
            ))
        }
    }
}
