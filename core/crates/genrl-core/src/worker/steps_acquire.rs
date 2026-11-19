use crate::error::{Error, Result};
use crate::http::Service;
use crate::jobs::{self, MoveOptions};
use crate::matching;
use crate::models::{Job, JobState};
use crate::qbit;
use crate::questions::{MatchOption, Payload};
use crate::settings::Settings;

use super::engine::Engine;
use super::types::{SizeGate, CATEGORY};

impl Engine {
    // ------------------------------------------------------------------ search

    pub(crate) async fn search_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let is_movie = job.media_kind.as_deref() == Some("movie");
        let is_series = job.media_kind.as_deref() == Some("series");
        if !is_movie && !is_series && !crate::metadata::is_resolved(&self.db, context.mal_id)? {
            let settings = Settings::load(&self.db)?;
            let store = crate::mal::KeyringStore;
            let mal = crate::mal::MalClient::new(
                &self.gate,
                settings.clone(),
                &store,
                self.clock.as_ref(),
            );
            let _ = crate::metadata::fetch_show_metadata_with_client(
                &self.db,
                &self.gate,
                &settings,
                self.clock.as_ref(),
                &self.data_dir,
                context.mal_id,
                true,
                &mal,
            )
            .await;

            if !crate::metadata::is_resolved(&self.db, context.mal_id)? {
                return self.block(
                    job,
                    "Metadata unresolved: refresh metadata before searching for releases",
                );
            }
        }
        let settings = Settings::load(&self.db)?;
        let (_inputs, search_names) = self.job_search_queries(job, &context, &settings)?;

        let is_movie = job.media_kind.as_deref() == Some("movie");
        let is_series = job.media_kind.as_deref() == Some("series");

        if !is_movie && !is_series && !self.gate.would_allow(Service::Nyaa) {
            return self.block(job, "Nyaa's circuit breaker is open");
        }

        let media_kind_opt = if is_movie {
            Some(crate::indexer::MediaKind::Movie)
        } else if is_series {
            Some(crate::indexer::MediaKind::Series)
        } else {
            None
        };

        let candidates = if let Some(kind) = media_kind_opt {
            match matching::search_candidates_for_kind(
                &self.gate,
                &settings,
                &search_names,
                1000,
                1000,
                kind,
            )
            .await
            {
                Ok(candidates) => candidates,
                Err(err) if err.is_transient() || err.is_connection_failure() => {
                    return self.block(job, &format!("indexer unreachable: {err}"));
                }
                Err(err) => return self.fail_or_reschedule(job, &format!("search failed: {err}")),
            }
        } else {
            match matching::search_candidates(&self.gate, &settings, &search_names, 1000, 1000)
                .await
            {
                Ok(candidates) => candidates,
                Err(err) if err.is_transient() || err.is_connection_failure() => {
                    return self.block(job, &format!("Nyaa unreachable: {err}"));
                }
                Err(err) => return self.fail_or_reschedule(job, &format!("search failed: {err}")),
            }
        };

        if candidates.is_empty() {
            return self.fail_or_reschedule(job, "no candidates on any of the show's names");
        }

        let now = self.now();
        for candidate in &candidates {
            jobs::remember_candidate(
                &self.db,
                self.clock.as_ref(),
                &jobs::RememberedCandidate {
                    info_hash: candidate.info_hash.clone(),
                    name: candidate.release.raw.clone(),
                    magnet: candidate.magnet.clone(),
                    seeders: candidate.seeders as i64,
                    leechers: candidate.leechers as i64,
                    size_bytes: candidate.size_bytes as i64,
                    group: candidate.release.group.clone(),
                    resolution: candidate.release.resolution.clone(),
                    source_url: None,
                    score: 0.0,
                    mal_id: if context.mal_id == 0 {
                        None
                    } else {
                        Some(context.mal_id)
                    },
                },
            )?;
        }
        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!(
                "found {} candidate{} for episode {} of {}",
                candidates.len(),
                if candidates.len() == 1 { "" } else { "s" },
                context.number,
                context.title
            ),
        )?;
        let _ = now;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Choose,
            "candidates remembered",
            &MoveOptions::default(),
        )
    }

    // ------------------------------------------------------------------ choose

    pub(crate) async fn choose_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let (inputs, rows, rules) = self.choose_inputs(job, &context)?;
        if rows.is_empty() {
            return self.fail_or_reschedule(job, "no candidates left to choose from");
        }

        let (ranked, rejected) = matching::rank(
            rows,
            &inputs,
            Some(context.number),
            &rules,
            &settings,
            self.now(),
        );

        // Scores back into the table, so the Queue page can show why.
        for entry in &ranked {
            self.db.with(|c| {
                c.execute(
                    "UPDATE torrents SET score = ?1 WHERE info_hash = ?2",
                    rusqlite::params![entry.scored.signal, entry.candidate.info_hash],
                )?;
                Ok(())
            })?;
        }

        let is_pack_grab = if job.media_kind.as_deref() == Some("movie") {
            false
        } else if job.media_kind.as_deref() == Some("series") {
            job.episode.is_none()
        } else {
            self.is_pack_grab(context.mal_id, &settings)?
        };

        // Free-space guard: pack candidates must fit in usable free space.
        let (ranked, none_fit) = if is_pack_grab {
            let (fitting, none_fit) = self.select_fitting_candidates(ranked, &settings);
            let filtered = if fitting.iter().any(|r| r.candidate.release.is_batch) {
                fitting
                    .into_iter()
                    .filter(|r| r.candidate.release.is_batch)
                    .collect()
            } else {
                fitting
            };
            (filtered, none_fit)
        } else {
            (ranked, false)
        };

        if none_fit {
            let download_path = crate::locations::effective_download_dir(&settings);
            let free_gb = self.disk.free_bytes(&download_path).unwrap_or(0) / (1024 * 1024 * 1024);
            let payload = Payload::DiskSpace {
                free_gb,
                floor_gb: settings.disk_floor_gb,
            };
            self.ask(
                job,
                context.mal_id,
                payload,
                "insufficient disk space for candidate packs",
            )?;
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::NeedsInput,
                "insufficient disk space for candidate packs",
                &MoveOptions::default(),
            );
        }

        if is_pack_grab
            && !ranked.is_empty()
            && !ranked.iter().any(|r| r.candidate.release.is_batch)
        {
            crate::events::EventLog::new(self.db.clone()).info(
                Some(job.id),
                Some(context.mal_id),
                "no suitable pack found; falling back to single episodes",
            )?;
        }

        // Only batches available while the show is still airing: ask.
        // Pack jobs intentionally look for packs, so they auto-select without asking BatchOnly.
        if !is_pack_grab
            && !inputs.fully_aired
            && !ranked.is_empty()
            && ranked.iter().all(|r| r.candidate.release.is_batch)
        {
            let best = &ranked[0];
            let payload = Payload::BatchOnly {
                release: best.candidate.release.raw.clone(),
                seeders: best.candidate.seeders,
            };
            self.set_chosen(job.id, &best.candidate.info_hash)?;
            self.ask(job, context.mal_id, payload, "only a batch is available")?;
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::NeedsInput,
                "only a batch is available",
                &MoveOptions::default(),
            );
        }

        match matching::decide_verdict(ranked, rejected, &inputs, &rules, &settings) {
            matching::Verdict::Auto { chosen, thin } => {
                self.set_chosen(job.id, &chosen.candidate.info_hash)?;
                let note = format!(
                    "chose {} (score {:.0}){}",
                    chosen.candidate.release.raw,
                    chosen.scored.signal,
                    if thin {
                        " with thin seeds, flagged"
                    } else {
                        ""
                    }
                );
                jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::Add,
                    &note,
                    &MoveOptions::default(),
                )
            }
            matching::Verdict::Ask {
                reason: matching::AskReason::ThinSeeds { seeders, chosen },
                ..
            } => {
                self.set_chosen(job.id, &chosen.candidate.info_hash)?;
                let payload = Payload::LowSeeds {
                    seeders,
                    release: chosen.candidate.release.raw.clone(),
                    magnet: chosen.candidate.magnet.clone(),
                    auto_list: inputs.auto,
                };
                self.ask(
                    job,
                    context.mal_id,
                    payload,
                    &format!("only {seeders} seeders"),
                )?;
                jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::NeedsInput,
                    &format!("only {seeders} seeders"),
                    &MoveOptions::default(),
                )
            }
            matching::Verdict::Ask { reason, ranked } => {
                let options: Vec<MatchOption> =
                    ranked.iter().take(4).map(|r| r.to_option()).collect();
                let payload = Payload::MatchReview {
                    expected: inputs.expected_show(),
                    options,
                    reason: reason.to_string(),
                };
                self.ask(job, context.mal_id, payload, &format!("{reason}"))?;
                jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::NeedsInput,
                    &format!("{reason}"),
                    &MoveOptions::default(),
                )
            }
            matching::Verdict::NotFound { rejections } => {
                if let Some(state) =
                    self.try_ask_unmapped_season(job, context.mal_id, &rejections)?
                {
                    return Ok(state);
                }

                let reasons = rejections
                    .iter()
                    .map(|(_, rejection)| rejection.to_string())
                    .collect::<Vec<_>>()
                    .join("; ");
                self.fail_or_reschedule(job, &format!("nothing usable: {reasons}"))
            }
        }
    }

    // --------------------------------------------------------------------- add

    pub(crate) async fn add_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;

        // Resources first: the design adds no torrent it cannot store.
        if let Some(reason) = self.blocking_reason(&settings) {
            return self.block(job, &reason);
        }

        let health = match qbit::ensure_running(&self.gate, &settings).await {
            Ok(health) => health,
            Err(err) if err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent is not reachable: {err}"));
            }
            Err(err) => {
                // One Inbox notice for an auth problem, not one per job.
                if matches!(err, Error::Auth { .. }) {
                    self.ensure_service_notice(context.mal_id, &err.to_string())?;
                }
                return self.block(job, &format!("qBittorrent is not usable: {err}"));
            }
        };

        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Choose,
                "no chosen candidate; choosing again",
                &MoveOptions::default(),
            );
        };

        match self.confirm_large_download(job, context.mal_id, &torrent)? {
            SizeGate::Approved => {}
            SizeGate::WaitingOnSibling => {
                // One card per pack: come back once the sibling's is answered.
                jobs::reschedule(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    self.now() + 60,
                    "waiting on the size confirmation for the same torrent",
                )?;
                return Ok(job.state);
            }
            SizeGate::Ask => return Ok(JobState::NeedsInput),
        }

        let download_path = crate::locations::effective_download_dir(&settings);
        if !download_path.exists() {
            return self.block(
                job,
                &format!("download folder '{}' is missing", download_path.display()),
            );
        }
        let check = crate::locations::check_directory(&download_path.to_string_lossy(), false);
        if !check.writable {
            return self.block(
                job,
                &format!(
                    "download folder '{}' is not writable",
                    download_path.display()
                ),
            );
        }
        let floor_bytes = settings.disk_floor_gb.saturating_mul(1024 * 1024 * 1024);
        let torrent_bytes = torrent.size_bytes.max(0) as u64;
        let required_bytes = torrent_bytes.saturating_add(floor_bytes);
        if let Some(free) = self.disk.free_bytes(&download_path) {
            if free < required_bytes {
                return self.block(job, &format!(
                    "download folder '{}' has insufficient space: need {} bytes (torrent {} + floor {}), free {} bytes",
                    download_path.display(),
                    required_bytes,
                    torrent_bytes,
                    floor_bytes,
                    free
                ));
            }
        }

        let dl_save_path = download_path.display().to_string();
        if let Err(err) = self
            .add_torrent_to_qbit(job, &torrent, &dl_save_path, CATEGORY, &settings)
            .await
        {
            if err.is_transient() || err.is_connection_failure() {
                return self.block(job, &format!("qBittorrent refused the add: {err}"));
            }
            return self.fail_or_reschedule(job, &format!("add failed: {err}"));
        }

        let is_batch = self.is_proven_batch_release(&torrent.name);

        if is_batch {
            self.handle_batch_add(job, context.mal_id, &torrent)?;
        } else {
            self.handle_single_add(job, &torrent)?;
            self.enqueue_sibling_episodes_if_needed(job, context.mal_id)?;
        }

        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!("added {} to qBittorrent {}", torrent.name, health.version),
        )?;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Monitor,
            &format!("added {}", torrent.name),
            &MoveOptions::default(),
        )
    }
}
