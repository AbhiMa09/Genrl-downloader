use crate::error::{Error, Result};
use crate::http::Service;
use crate::jobs::{self, MoveOptions};
use crate::matching::{self, TargetInputs};
use crate::models::{Job, JobState};
use crate::qbit;
use crate::questions::{MatchOption, Payload};
use crate::rules::Rules;
use crate::settings::Settings;

use super::engine::Engine;
use super::types::CATEGORY;

impl Engine {
    // ------------------------------------------------------------------ search

    pub(crate) async fn search_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let inputs = TargetInputs::load(&self.db, context.mal_id)?;

        // A service that is out is not a failure: wait for the breaker. The
        // pre-check is non-consuming, so when the cooldown elapses the next
        // request claims the half-open probe and closes it on success.
        if !self.gate.would_allow(Service::Nyaa) {
            return self.block(job, "Nyaa's circuit breaker is open");
        }

        let rules = Rules::for_show(&self.db, context.mal_id)?;
        let mut search_names = inputs.names.clone();
        if inputs.fully_aired && rules.prefer_batch_when_finished {
            let mut batch_queries = Vec::new();
            for n in &inputs.names {
                batch_queries.push(format!("{n} Batch"));
                batch_queries.push(format!("{n} Season {}", inputs.season));
            }
            batch_queries.extend(search_names);
            search_names = batch_queries;
        }

        let candidates =
            match matching::search_candidates(&self.gate, &settings, &search_names, 50, 25).await {
                Ok(candidates) => candidates,
                Err(err) if err.is_transient() || err.is_connection_failure() => {
                    return self.block(job, &format!("Nyaa unreachable: {err}"));
                }
                Err(err) => return self.fail_or_reschedule(job, &format!("search failed: {err}")),
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
                    mal_id: Some(context.mal_id),
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
        let rules = Rules::for_show(&self.db, context.mal_id)?;
        let inputs = TargetInputs::load(&self.db, context.mal_id)?;

        let rows = self.candidate_rows(context.mal_id)?;
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

        // Only batches available while the show is still airing: ask.
        if !inputs.fully_aired
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
            return Ok(JobState::NeedsInput);
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
                reason: matching::AskReason::ThinSeeds { seeders },
                ranked,
            } => {
                let best = &ranked[0];
                self.set_chosen(job.id, &best.candidate.info_hash)?;
                let payload = Payload::LowSeeds {
                    seeders,
                    release: best.candidate.release.raw.clone(),
                    magnet: best.candidate.magnet.clone(),
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

        let dl_str = if settings.download_folder.trim().is_empty() {
            crate::locations::default_download_dir()
                .display()
                .to_string()
        } else {
            settings.download_folder.clone()
        };
        let download_path = crate::locations::resolve_home_dir(&dl_str);
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
        let client = qbit::Qbit::from_settings(&settings)?;
        match client
            .add(
                &self.gate,
                std::slice::from_ref(&torrent.magnet),
                Some(&dl_save_path),
                Some(CATEGORY),
            )
            .await
        {
            Ok(()) => {}
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent refused the add: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("add failed: {err}")),
        }

        let is_batch = torrent.name.to_ascii_lowercase().contains("batch")
            || crate::filename::parse_release(&torrent.name).is_batch;

        if is_batch {
            self.db.with(|c| {
                c.execute(
                    "UPDATE episodes SET state = 'downloading', torrent_id = (
                         SELECT id FROM torrents WHERE info_hash = ?1
                     )
                     WHERE mal_id = ?2 AND state IN ('missing', 'queued')",
                    rusqlite::params![torrent.info_hash, context.mal_id],
                )?;
                c.execute(
                    "DELETE FROM jobs WHERE id != ?1 AND episode_id IN (
                         SELECT id FROM episodes WHERE mal_id = ?2
                     ) AND kind = 'search'",
                    rusqlite::params![job.id, context.mal_id],
                )?;
                Ok(())
            })?;
        } else {
            self.enqueue_sibling_episodes_if_needed(context.mal_id, job.episode_id)?;
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
