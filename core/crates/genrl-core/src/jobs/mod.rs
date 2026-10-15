//! The job queue's state machine.
//!
//! One table of allowed moves and one [`transition`] function enforce the
//! design's diagram — five main steps left to right (search → choose → add →
//! monitor → finalize), three side states (needs_input, blocked, failed), and
//! the two loops back: a stalled torrent returns to `search` for the next
//! candidate, and a blocked job returns to whatever state it was paused in.
//!
//! Two rules shape everything here:
//!
//! 1. **A transition is one transaction, written before the work it enables.**
//!    [`transition`] commits the new state *and* its Activity-log line in the
//!    same statement batch, so a crash can never leave a state change without
//!    its record — and the step that runs afterwards does so on the strength
//!    of state that is already durable.
//! 2. **A job's state means "about to run this step's work".** That is what
//!    makes every step idempotent and the crash-recovery check possible: a
//!    job killed mid-step is simply re-run, because the step's side effects
//!    are keyed (episodes unique on `(mal_id, number)`, torrents unique on
//!    `info_hash`).

pub mod backoff;
pub mod enqueue;
pub mod queries;
pub mod transitions;

pub use backoff::*;
pub use enqueue::*;
pub use queries::*;
pub use transitions::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::clock::{Clock, FakeClock};
    use crate::db::Db;
    use crate::error::Error;
    use crate::events::EventLog;
    use crate::models::{JobKind, JobState};

    fn db() -> Db {
        Db::open_in_memory().unwrap()
    }

    fn clock() -> FakeClock {
        FakeClock::new(1_789_227_900)
    }

    /// A show and an episode to hang a job off.
    fn episode(db: &Db, clock: &dyn Clock) -> i64 {
        crate::planner::upsert_show(
            db,
            &crate::planner::ListEntry {
                mal_id: 1,
                title: "Show".into(),
                english_title: None,
                list_status: crate::models::ListStatus::Watching,
                watched_count: 0,
                total_episodes: Some(12),
                start_year: Some(2025),
                auto: false,
                episode_offset: 0,
                names: vec!["Show".into()],
                franchise: 1,
                season: 1,
            },
        )
        .unwrap();
        crate::planner::ensure_episode(db, clock, 1, 5.0).unwrap()
    }

    fn state_of(db: &Db, job_id: i64) -> JobState {
        get(db, job_id).unwrap().unwrap().state
    }

    #[test]
    fn every_documented_move_is_allowed() {
        for (from, to) in TRANSITIONS {
            assert!(
                is_allowed(*from, *to),
                "the table lists {from} → {to}, so it must be allowed"
            );
        }
    }

    #[test]
    fn moves_off_the_diagram_are_refused() {
        // The forbidden directions that matter: backwards along the main line,
        // skipping steps, and leaving a terminal state.
        let forbidden = [
            (JobState::Search, JobState::Add),
            (JobState::Search, JobState::Monitor),
            (JobState::Search, JobState::Finalize),
            (JobState::Search, JobState::Done),
            (JobState::Choose, JobState::Monitor),
            (JobState::Add, JobState::Finalize),
            (JobState::Add, JobState::Search),
            (JobState::Monitor, JobState::Choose),
            (JobState::Finalize, JobState::Add),
            (JobState::NeedsInput, JobState::Blocked),
            (JobState::Blocked, JobState::NeedsInput),
            (JobState::Failed, JobState::Add),
            (JobState::Done, JobState::Search),
            (JobState::Done, JobState::Failed),
        ];
        for (from, to) in forbidden {
            assert!(!is_allowed(from, to), "{from} → {to} must not be allowed");
        }
    }

    #[test]
    fn staying_put_is_not_a_transition() {
        for state in [JobState::Search, JobState::Add, JobState::Blocked] {
            assert!(!is_allowed(state, state), "{state} → {state} is not a move");
        }
    }

    #[test]
    fn a_legal_move_lands_and_logs_in_one_transaction() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        assert_eq!(state_of(&db, job), JobState::Search);

        let from = transition(
            &db,
            &clock,
            job,
            JobState::Choose,
            "candidates remembered",
            &MoveOptions::default(),
        )
        .unwrap();
        assert_eq!(from, JobState::Search);
        assert_eq!(state_of(&db, job), JobState::Choose);

        // The transition and its Activity line are one fact.
        let events = EventLog::new(db.clone()).recent(5).unwrap();
        assert_eq!(events.len(), 2, "queued, then the move");
        assert!(events[0].message.contains("search → choose"), "{:?}", events[0]);
        assert!(events[1].message.contains("queued"), "{:?}", events[1]);
    }

    #[test]
    fn an_illegal_move_is_an_error_and_changes_nothing() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        let err = transition(
            &db,
            &clock,
            job,
            JobState::Monitor,
            "skip a step",
            &MoveOptions::default(),
        )
        .unwrap_err();
        assert!(
            matches!(err, Error::IllegalTransition { from, to, .. } if from == JobState::Search && to == JobState::Monitor),
            "{err:?}"
        );
        assert_eq!(state_of(&db, job), JobState::Search, "nothing moved");
    }

    #[test]
    fn a_done_job_never_moves_again() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        for to in [
            JobState::Choose,
            JobState::Add,
            JobState::Monitor,
            JobState::Finalize,
            JobState::Done,
        ] {
            transition(&db, &clock, job, to, "walk it home", &MoveOptions::default()).unwrap();
        }
        assert_eq!(state_of(&db, job), JobState::Done);
        let err = transition(
            &db,
            &clock,
            job,
            JobState::Search,
            "back from the dead",
            &MoveOptions::default(),
        )
        .unwrap_err();
        assert!(err.to_string().contains("never leaves"), "{err}");
    }

    #[test]
    fn blocked_remembers_where_it_returns_to() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        transition(&db, &clock, job, JobState::Choose, "candidates", &MoveOptions::default()).unwrap();
        transition(&db, &clock, job, JobState::Add, "added", &MoveOptions::default()).unwrap();
        transition(
            &db,
            &clock,
            job,
            JobState::Blocked,
            "qBittorrent is down",
            &MoveOptions::blocked(JobState::Add),
        )
        .unwrap();

        let stored = get(&db, job).unwrap().unwrap();
        assert_eq!(stored.state, JobState::Blocked);
        assert_eq!(stored.blocked_from, Some(JobState::Add));
        // The kind follows the step, so the download limiter still applies.
        assert_eq!(stored.kind, JobKind::Add);

        transition(&db, &clock, job, JobState::Add, "back again", &MoveOptions::default()).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().blocked_from, None);
    }

    #[test]
    fn kind_follows_the_step() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        let expect = [
            (JobState::Choose, JobKind::Search),
            (JobState::Add, JobKind::Add),
            (JobState::Monitor, JobKind::Monitor),
            (JobState::Finalize, JobKind::Finalize),
            (JobState::Blocked, JobKind::Finalize),
            (JobState::Search, JobKind::Search),
        ];
        for (to, kind) in expect {
            let opts = if to == JobState::Blocked {
                MoveOptions::blocked(JobState::Finalize)
            } else {
                MoveOptions::default()
            };
            transition(&db, &clock, job, to, "walk", &opts).unwrap();
            assert_eq!(get(&db, job).unwrap().unwrap().kind, kind);
        }
    }

    #[test]
    fn attempts_count_failures_and_reset_on_retry() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        transition(&db, &clock, job, JobState::Failed, "no candidates", &MoveOptions { count_attempt: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 1);
        transition(&db, &clock, job, JobState::Search, "retry", &MoveOptions::default()).unwrap();
        transition(&db, &clock, job, JobState::Failed, "again", &MoveOptions { count_attempt: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 2);
        transition(&db, &clock, job, JobState::Search, "fresh attempt", &MoveOptions { reset_attempts: true, ..Default::default() }).unwrap();
        assert_eq!(get(&db, job).unwrap().unwrap().attempts, 0);
    }

    #[test]
    fn enqueueing_the_same_episode_twice_is_a_no_op() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let first = enqueue_search(&db, &clock, episode).unwrap();
        let second = enqueue_search(&db, &clock, episode).unwrap();
        assert_eq!(first, second);

        let count: i64 = db
            .with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn a_finished_episode_can_be_replanned_without_a_second_job() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();
        for to in [JobState::Choose, JobState::Add, JobState::Monitor, JobState::Finalize, JobState::Done] {
            transition(&db, &clock, job, to, "run", &MoveOptions::default()).unwrap();
        }
        let replanned = enqueue_search(&db, &clock, episode).unwrap();
        assert_ne!(replanned, job, "a done job is never reopened");
        assert_eq!(state_of(&db, replanned), JobState::Search);
    }

    #[test]
    fn candidates_deduplicate_by_info_hash() {
        let db = db();
        let clock = clock();
        episode(&db, &clock);
        let candidate = || RememberedCandidate {
            info_hash: "abc".repeat(13) + "abcd",
            name: "[SubsPlease] Show - 05 (1080p).mkv".to_string(),
            magnet: "magnet:?xt=urn:btih:abc".to_string(),
            seeders: 20,
            leechers: 1,
            size_bytes: 1_500_000_000,
            group: Some("SubsPlease".to_string()),
            resolution: Some("1080p".to_string()),
            source_url: None,
            score: 90.0,
            mal_id: Some(1),
        };
        let first = remember_candidate(&db, &clock, &candidate()).unwrap();
        let mut changed = candidate();
        changed.seeders = 99;
        let second = remember_candidate(&db, &clock, &changed).unwrap();
        assert_eq!(first, second);

        let (rows, seeders): (i64, i64) = db
            .with(|c| {
                Ok(c.query_row(
                    "SELECT COUNT(*), COALESCE(MAX(seeders), 0) FROM torrents",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .unwrap();
        assert_eq!((rows, seeders), (1, 99));
    }

    #[test]
    fn due_jobs_respect_their_time() {
        let db = db();
        let clock = clock();
        let episode = episode(&db, &clock);
        let job = enqueue_search(&db, &clock, episode).unwrap();

        assert_eq!(due_jobs(&db, clock.now(), &[], 10).unwrap().len(), 1);
        reschedule(&db, &clock, job, clock.now() + 600, "not yet").unwrap();
        assert!(due_jobs(&db, clock.now(), &[], 10).unwrap().is_empty());
        clock.advance(601);
        assert_eq!(due_jobs(&db, clock.now(), &[], 10).unwrap().len(), 1);
        let error = get(&db, job).unwrap().unwrap().last_error.unwrap();
        assert_eq!(error, "not yet");
    }

    #[test]
    fn the_backoff_ladder_matches_the_design() {
        assert_eq!(backoff_delay(1), Duration::from_secs(5));
        assert_eq!(backoff_delay(2), Duration::from_secs(30));
        assert_eq!(backoff_delay(3), Duration::from_secs(120));
        assert_eq!(backoff_delay(4), Duration::from_secs(600));
        assert_eq!(backoff_delay(5), Duration::from_secs(3600));
        assert_eq!(backoff_delay(50), Duration::from_secs(3600), "then hold");
    }

    #[test]
    fn jitter_spreads_a_delay_by_up_to_a_quarter() {
        let base = Duration::from_secs(100);
        assert_eq!(with_jitter(base, 0.0), base);
        assert_eq!(with_jitter(base, 1.0), Duration::from_secs(125));
        assert!(with_jitter(base, 0.5) > base && with_jitter(base, 0.5) < Duration::from_secs(125));
    }

    #[test]
    fn the_daily_cap_counts_from_the_start_of_the_day() {
        assert_eq!(start_of_day(0), 0);
        assert_eq!(start_of_day(86_399), 0);
        assert_eq!(start_of_day(86_400), 86_400);
        let midday = 86_400 + 3_600;
        assert_eq!(start_of_day(midday), 86_400);
    }
}
