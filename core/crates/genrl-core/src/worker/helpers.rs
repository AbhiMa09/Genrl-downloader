use rusqlite::OptionalExtension;

use crate::error::Result;
use crate::jobs::{self, MoveOptions};
use crate::models::{Job, JobState};
use crate::questions::{Payload, QuestionStore};
use crate::score::Candidate;
use crate::settings::Settings;

use super::engine::Engine;
use super::inbox::tried;
use super::types::ChosenTorrent;

impl Engine {
    // ------------------------------------------------------------------ helpers

    /// If single-episode download is used, ensure sibling missing episodes have search jobs queued.
    pub(crate) fn enqueue_sibling_episodes_if_needed(
        &self,
        mal_id: i64,
        current_episode_id: i64,
    ) -> Result<()> {
        let missing_ids: Vec<i64> = self.db.with(|c| {
            let mut stmt = c.prepare(
                "SELECT e.id FROM episodes e
                 WHERE e.mal_id = ?1 AND e.id != ?2 AND e.state IN ('missing', 'queued')
                    AND NOT EXISTS (
                        SELECT 1 FROM jobs j WHERE j.episode_id = e.id AND j.kind = 'search'
                    )
                 ORDER BY e.number ASC",
            )?;
            let rows =
                stmt.query_map(rusqlite::params![mal_id, current_episode_id], |r| r.get(0))?;
            let mut list = Vec::new();
            for r in rows {
                list.push(r?);
            }
            Ok(list)
        })?;

        for ep_id in missing_ids {
            crate::jobs::enqueue_search(&self.db, self.clock.as_ref(), ep_id)?;
        }
        Ok(())
    }

    /// Back off, or give up and ask: whichever comes first.
    pub(crate) fn fail_or_reschedule(&self, job: &Job, reason: &str) -> Result<JobState> {
        let settings = Settings::load(&self.db)?;
        if job.attempts + 1 >= i64::from(settings.max_failed_candidates) {
            let payload = Payload::AllFailed {
                candidates_tried: tried(job.attempts + 1),
            };
            self.ask(job, self.episode_mal_id(job)?, payload, reason)?;
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Failed,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    ..Default::default()
                },
            )
        } else {
            jobs::reschedule(
                &self.db,
                self.clock.as_ref(),
                job.id,
                self.now() + jobs::backoff_delay(job.attempts + 1).as_secs() as i64,
                reason,
            )?;
            Ok(job.state)
        }
    }

    /// Stall or vanished torrent: move to the next candidate, or ask.
    pub(crate) fn next_candidate(&self, job: &Job, mal_id: i64, reason: &str) -> Result<JobState> {
        let settings = Settings::load(&self.db)?;
        if job.attempts + 1 >= i64::from(settings.max_failed_candidates) {
            let payload = Payload::AllFailed {
                candidates_tried: tried(job.attempts + 1),
            };
            // Out of candidates and out of attempts: a failed job, and — per
            // the design — a failed job raises a question too.
            self.ask(job, mal_id, payload, reason)?;
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Failed,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    clear_torrent: true,
                    ..Default::default()
                },
            )
        } else {
            jobs::transition(
                &self.db,
                self.clock.as_ref(),
                job.id,
                JobState::Search,
                reason,
                &MoveOptions {
                    count_attempt: true,
                    clear_torrent: true,
                    ..Default::default()
                },
            )
        }
    }

    /// Queue a question and park the job in `needs_input`.
    pub(crate) fn ask(
        &self,
        job: &Job,
        mal_id: i64,
        payload: Payload,
        reason: &str,
    ) -> Result<i64> {
        let settings = Settings::load(&self.db)?;
        let now = self.now();
        let id = QuestionStore::ask(
            &self.db,
            Some(job.id),
            Some(mal_id),
            &payload,
            now,
            Some(i64::from(settings.question_expiry_hours) * 3600),
        )?;
        // The reason is the Activity line: every ask is visible.
        crate::events::EventLog::new(self.db.clone()).warn(
            Some(job.id),
            Some(mal_id),
            format!("asked a question (#{id}): {reason}"),
        )?;
        Ok(id)
    }

    /// One Inbox notice per service, never one per job.
    pub(crate) fn ensure_service_notice(&self, mal_id: i64, detail: &str) -> Result<()> {
        let existing = QuestionStore::of_kind(
            &self.db,
            crate::models::QuestionKind::ServiceNotice,
            self.now(),
        )?;
        if existing
            .iter()
            .any(|question| matches!(&question.payload, Payload::ServiceNotice { service, .. } if service == "qBittorrent"))
        {
            return Ok(());
        }
        QuestionStore::ask(
            &self.db,
            None,
            Some(mal_id),
            &Payload::ServiceNotice {
                service: "qBittorrent".to_string(),
                detail: detail.to_string(),
            },
            self.now(),
            None,
        )?;
        Ok(())
    }

    pub(crate) fn episode_mal_id(&self, job: &Job) -> Result<i64> {
        self.db.with(|c| {
            Ok(c.query_row(
                "SELECT mal_id FROM episodes WHERE id = ?1",
                [job.episode_id],
                |r| r.get(0),
            )?)
        })
    }

    pub(crate) fn chosen_torrent(&self, job: &Job) -> Result<Option<ChosenTorrent>> {
        let Some(torrent_id) = job.torrent_id else {
            return Ok(None);
        };
        self.db.with(|c| {
            Ok(c.query_row(
                "SELECT info_hash, name, magnet, size FROM torrents WHERE id = ?1",
                [torrent_id],
                |r| {
                    Ok(ChosenTorrent {
                        info_hash: r.get(0)?,
                        name: r.get(1)?,
                        magnet: r.get(2)?,
                        size_bytes: r.get::<_, i64>(3).unwrap_or(0),
                    })
                },
            )
            .optional()?)
        })
    }

    /// The show's non-rejected candidates, newest first, as scoring inputs.
    pub(crate) fn candidate_rows(&self, mal_id: i64) -> Result<Vec<Candidate>> {
        self.db.with_rusqlite(|c| {
            let mut stmt = c.prepare(
                "SELECT name, info_hash, magnet, seeders, leechers, size, \"group\",
                        resolution, trusted, found_at
                 FROM torrents WHERE mal_id = ?1 AND rejected = 0
                 ORDER BY found_at DESC, seeders DESC LIMIT 50",
            )?;
            let rows = stmt.query_map([mal_id], |r| {
                let name: String = r.get(0)?;
                let release = crate::filename::parse_release(&name);
                Ok(Candidate {
                    release,
                    info_hash: r.get(1)?,
                    magnet: r.get(2)?,
                    seeders: r.get::<_, i64>(3)? as u32,
                    leechers: r.get::<_, i64>(4)? as u32,
                    trusted: r.get::<_, i64>(8)? != 0,
                    remake: false,
                    size_bytes: r.get::<_, i64>(5)? as u64,
                    published_at: r.get(9)?,
                    uploader: None,
                    show_type: None,
                })
            })?;
            let mut out = Vec::new();
            for row in rows {
                out.push(row?);
            }
            Ok(out)
        })
    }
}
