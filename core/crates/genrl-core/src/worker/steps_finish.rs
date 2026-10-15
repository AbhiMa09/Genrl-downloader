use std::path::Path;

use crate::error::{Error, Result};
use crate::jobs::{self, MoveOptions};
use crate::models::{Job, JobState};
use crate::planner;
use crate::qbit;
use crate::settings::Settings;

use super::engine::Engine;
use super::types::{FinalizeContext, FinalizeOutcome, MONITOR_POLL_SECS};

impl Engine {
    // ----------------------------------------------------------------- monitor

    pub(crate) async fn monitor_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Choose,
                "no torrent to monitor; choosing again",
                &MoveOptions::default(),
            );
        };

        let client = qbit::Qbit::from_settings(&settings)?;
        let info = match client.get_torrent(&self.gate, &torrent.info_hash).await {
            Ok(info) => info,
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent unreachable while monitoring: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("monitor failed: {err}")),
        };

        let Some(info) = info else {
            // Gone from qBittorrent (removed by hand, say): try the next one.
            return self.next_candidate(job, context.mal_id, "the torrent left qBittorrent");
        };

        if info.is_complete() {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Finalize,
                &format!("{} finished", info.name),
                &MoveOptions::default(),
            );
        }

        let moved = info.progress > job.progress + f64::EPSILON;
        jobs::note_progress(&self.db, self.clock.as_ref(), job.id, info.progress, moved)?;
        if !moved {
            let since = job.last_progress_at.unwrap_or(job.updated_at);
            let stalled_for = self.now() - since;
            let timeout = if info.num_seeds == 0 {
                // Torrent has 0 seeders: fail over quickly (2 mins) to single episodes or next candidate
                120
            } else {
                i64::from(settings.stall_timeout_min) * 60
            };
            if stalled_for >= timeout {
                // Remove it, mark the candidate tried, and look for the next.
                let _ = client.remove(&self.gate, &torrent.info_hash, false).await;
                jobs::reject_candidate(&self.db, &torrent.info_hash)?;
                let reason = if info.num_seeds == 0 {
                    format!("0 seeders detected for {}; switched to next candidate/single episodes", info.name)
                } else {
                    format!("no progress for {} minutes; removed {} and trying the next candidate", settings.stall_timeout_min, info.name)
                };
                return self.next_candidate(
                    job,
                    context.mal_id,
                    &reason,
                );
            }
        }

        jobs::reschedule(
            &self.db,
            self.clock.as_ref(),
            job.id,
            self.now() + MONITOR_POLL_SECS,
            &format!(
                "{} at {:.0}%",
                info.state,
                info.progress * 100.0
            ),
        )?;
        Ok(JobState::Monitor)
    }

    // ---------------------------------------------------------------- finalize

    pub(crate) async fn finalize_step(&self, job: &Job) -> Result<JobState> {
        let context = self.context(job)?;
        let settings = Settings::load(&self.db)?;
        let Some(torrent) = self.chosen_torrent(job)? else {
            return jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Search,
                "nothing to finalize; searching again",
                &MoveOptions::default(),
            );
        };

        let client = qbit::Qbit::from_settings(&settings)?;
        let info = match client.get_torrent(&self.gate, &torrent.info_hash).await {
            Ok(info) => info,
            Err(err) if err.is_transient() || err.is_connection_failure() => {
                return self.block(job, &format!("qBittorrent unreachable while verifying: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("verify failed: {err}")),
        };

        // The file on disk is the fact that matters; qBittorrent forgetting the
        // torrent must not lose a finished download.
        let (content_path, size_bytes) = match &info {
            Some(info) if info.is_complete() => {
                (info.content_path.clone(), info.size.max(0))
            }
            Some(_) => {
                return jobs::reschedule(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    self.now() + MONITOR_POLL_SECS,
                    "still downloading; waiting",
                )
                .map(|()| JobState::Finalize);
            }
            None => {
                let path = Path::new(&settings.download_folder).join(&torrent.name);
                match std::fs::metadata(&path) {
                    Ok(meta) if meta.len() > 0 => {
                        (path.display().to_string(), meta.len() as i64)
                    }
                    _ => {
                        return jobs::transition(
                            &self.db,
                            self.clock.as_ref(),
                            job.id,
                            JobState::Search,
                            "the finished file is gone; searching again",
                            &MoveOptions::default(),
                        )
                    }
                }
            }
        };

        // Verify: size matches, and the extension is a video type.
        let path = Path::new(&content_path);
        let verified = if path.is_dir() {
            std::fs::read_dir(path).map(|mut d| d.next().is_some()).unwrap_or(false)
        } else {
            std::fs::metadata(&content_path)
                .map(|meta| meta.len() as i64 == size_bytes.max(0) || size_bytes == 0)
                .unwrap_or(false)
        };
        if !verified {
            return self.fail_or_reschedule(
                job,
                &format!("{content_path} does not verify against the torrent"),
            );
        }

        let lib_root = if settings.library_folder.trim().is_empty() {
            crate::locations::default_library_dir()
        } else {
            crate::locations::resolve_home_dir(&settings.library_folder)
        };
        let (title, start_year) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT title, start_year FROM shows WHERE mal_id = ?1",
                [context.mal_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)),
            )?)
        })?;
        let dest_show_dir = crate::library::resolve_show_folder(
            &self.db,
            &lib_root,
            context.mal_id,
            &title,
            start_year,
        )?;

        if !dest_show_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&dest_show_dir) {
                return self.block(job, &format!("destination folder '{}' could not be created: {e}", dest_show_dir.display()));
            }
        }
        let check = crate::locations::check_directory(&dest_show_dir.to_string_lossy(), false);
        if !check.writable {
            return self.block(job, &format!("destination folder '{}' is not writable", dest_show_dir.display()));
        }
        let floor_bytes = settings.disk_floor_gb.saturating_mul(1024 * 1024 * 1024);
        let required_bytes = (size_bytes.max(0) as u64).saturating_add(floor_bytes);
        if let Some(free) = self.disk.free_bytes(&dest_show_dir) {
            if free < required_bytes {
                return self.block(job, &format!(
                    "destination folder '{}' has insufficient space: need {} bytes, free {} bytes",
                    dest_show_dir.display(),
                    required_bytes,
                    free
                ));
            }
        }

        let outcome = self
            .finalizer
            .finalize(&FinalizeContext {
                job,
                episode_id: job.episode_id,
                mal_id: context.mal_id,
                content_path: &content_path,
                size_bytes,
            })
            .await;
        let dest_path = match outcome {
            Ok(FinalizeOutcome::Done(success)) => success.destination_path,
            Ok(FinalizeOutcome::NeedsInput { .. }) => {
                return jobs::transition(
                    &self.db,
                    self.clock.as_ref(),
                    job.id,
                    JobState::NeedsInput,
                    "destination file exists; waiting for user choice",
                    &MoveOptions::default(),
                );
            }
            Err(err) if err.is_transient() || matches!(err, Error::Io(_)) => {
                return self.block(job, &format!("library step could not file it: {err}"));
            }
            Err(err) => return self.fail_or_reschedule(job, &format!("finalize failed: {err}")),
        };

        planner::mark_done(&self.db, job.episode_id, &dest_path, size_bytes)?;

        if settings.remove_when_finished {
            let client = qbit::Qbit::from_settings(&settings)?;
            let _ = client.remove(&self.gate, &torrent.info_hash, false).await;
        }

        crate::events::EventLog::new(self.db.clone()).info(
            Some(job.id),
            Some(context.mal_id),
            format!("verified {} and filed it to {}", content_path, dest_path),
        )?;

        jobs::transition(
            &self.db,
            self.clock.as_ref(),
            job.id,
            JobState::Done,
            "episode owned",
            &MoveOptions::default(),
        )
    }
}
