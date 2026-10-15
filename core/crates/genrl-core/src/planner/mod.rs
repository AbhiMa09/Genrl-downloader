//! The planner: turning the user's MAL list into episode rows and jobs.
//!
//! The list decides what is wanted (the design's season and watch-status
//! rules); this module turns that decision into rows. Three rules shape it:
//!
//! * **Nothing is added before the first sync is confirmed.** A first sync
//!   produces a [`PlanPreview`] — "this will queue 214 episodes across 31
//!   shows" — and waits for one confirmation, so a bad import cannot queue
//!   hundreds of episodes at once.
//! * **A daily cap on new torrents** keeps a bad sync from flooding the
//!   client; a first sync is capped tighter still.
//! * **Only episodes up to the aired count**, and re-checking weekly, so an
//!   airing show gains new episodes without anything being queued twice
//!   (`episodes` is unique on `(mal_id, number)` and jobs are idempotent).
//!
//! Until MAL OAuth lands (phase 5) the list comes from a JSON file, which is
//! also how the tests drive this module.

pub mod apply;
pub mod db_ops;
pub mod list;
pub mod preview;

pub use apply::*;
pub use db_ops::*;
pub use list::*;
pub use preview::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use crate::models::ListStatus;
    use crate::settings::Settings;

    fn clock() -> crate::clock::FakeClock {
        crate::clock::FakeClock::new(1_789_227_900)
    }

    /// S1 finished, S2 four episodes in: the design's fixture chain.
    fn list() -> MalList {
        MalList {
            entries: vec![
                ListEntry {
                    mal_id: 100,
                    title: "Show".to_string(),
                    english_title: None,
                    list_status: ListStatus::Completed,
                    watched_count: 12,
                    total_episodes: Some(12),
                    start_year: Some(2024),
                    auto: false,
                    episode_offset: 0,
                    names: vec!["Show".to_string()],
                    franchise: 1,
                    season: 1,
                },
                ListEntry {
                    mal_id: 200,
                    title: "Show 2".to_string(),
                    english_title: Some("Show Two".to_string()),
                    list_status: ListStatus::Watching,
                    watched_count: 4,
                    total_episodes: Some(12),
                    start_year: Some(2025),
                    auto: true,
                    episode_offset: 0,
                    names: vec!["Show 2".to_string()],
                    franchise: 1,
                    season: 2,
                },
            ],
        }
    }

    fn first_sync_done(db: &Db) {
        let mut settings = Settings::load(db).unwrap();
        settings.first_run_complete = true;
        Settings::save(&settings, db).unwrap();
    }

    #[test]
    fn a_first_sync_previews_and_adds_nothing() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let outcome = plan(&db, &clock, &list(), false).unwrap();

        let PlanOutcome::Preview(preview) = outcome else {
            panic!("the first sync must only preview");
        };
        assert!(preview.awaiting_confirmation);
        assert_eq!(preview.episodes.len(), 1, "only S2 wants an episode");
        assert_eq!(preview.episodes[0].episode, 5.0);
        assert_eq!(preview.episodes[0].season, 2);
        assert!(preview.summary().contains("1 episode across 1 show"));

        // Nothing written: no shows, no episodes, no jobs.
        let shows: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM shows", [], |r| r.get(0))?)).unwrap();
        let episodes: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM episodes", [], |r| r.get(0))?)).unwrap();
        let jobs: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?)).unwrap();
        assert_eq!((shows, episodes, jobs), (0, 0, 0));

        // The preview waits in settings for the one confirmation.
        assert!(pending_preview(&db).unwrap().is_some());

        let queued = confirm(&db, &clock, &list()).unwrap();
        assert_eq!(queued, 1, "confirmation applies exactly what was previewed");
        assert!(pending_preview(&db).unwrap().is_none());
    }

    #[test]
    fn confirming_a_second_time_does_nothing() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        assert!(matches!(plan(&db, &clock, &list(), false).unwrap(), PlanOutcome::Preview(_)));
        assert_eq!(confirm(&db, &clock, &list()).unwrap(), 1);
        assert_eq!(confirm(&db, &clock, &list()).unwrap(), 0, "no preview left");
    }

    #[test]
    fn a_later_sync_applies_immediately_and_is_idempotent() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);

        let PlanOutcome::Applied { queued, .. } = plan(&db, &clock, &list(), false).unwrap() else {
            panic!("after the first sync, planning applies");
        };
        assert_eq!(queued, 1);

        // Same list again: the episode and the job already exist.
        let again = plan(&db, &clock, &list(), true).unwrap();
        assert!(
            matches!(&again, PlanOutcome::Applied { queued: 0, .. }),
            "the weekly re-check must be a no-op: {again:?}"
        );
        let jobs: i64 = db.with(|c| Ok(c.query_row("SELECT COUNT(*) FROM jobs", [], |r| r.get(0))?)).unwrap();
        assert_eq!(jobs, 1, "duplicate enqueue is a no-op");
    }

    #[test]
    fn the_weekly_cadence_holds_until_it_passes() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);

        assert!(matches!(plan(&db, &clock, &list(), false).unwrap(), PlanOutcome::Applied { .. }));
        // Mark S2E5 owned so the next plan has nothing new to offer.
        db.with(|c| {
            c.execute("UPDATE episodes SET state = 'done' WHERE number = 5", [])?;
            Ok(())
        })
        .unwrap();
        db.with(|c| {
            c.execute("UPDATE shows SET auto = 0 WHERE mal_id = 100", [])?;
            Ok(())
        })
        .unwrap();

        clock.advance(24 * 3600);
        let too_soon = plan(&db, &clock, &list(), false).unwrap();
        assert!(
            matches!(&too_soon, PlanOutcome::Preview(p) if p.episodes.is_empty() && !p.awaiting_confirmation),
            "inside a week the planner stays quiet: {too_soon:?}"
        );

        clock.advance(6 * 24 * 3600);
        let forced = plan(&db, &clock, &list(), true).unwrap();
        assert!(matches!(forced, PlanOutcome::Applied { .. }), "force ignores the cadence");
    }

    #[test]
    fn plan_to_watch_is_skipped_in_the_preview_with_a_reason() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let mut planned = list();
        planned.entries[1].list_status = ListStatus::PlanToWatch;
        let outcome = plan(&db, &clock, &planned, false).unwrap();
        let PlanOutcome::Preview(preview) = outcome else {
            panic!("first sync previews");
        };
        assert!(preview.episodes.is_empty());
        assert_eq!(preview.skipped.len(), 2, "every entry in the chain is reported");
        assert!(preview.skipped[1].reason.contains("plan to watch"), "{:?}", preview.skipped);
    }

    #[test]
    fn only_episodes_up_to_the_aired_count_are_queued() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        first_sync_done(&db);
        let mut planned = list();
        // S2 has five aired; the user has watched none of them: only 1 is next.
        planned.entries[1].watched_count = 0;
        planned.entries[1].total_episodes = Some(5);
        let PlanOutcome::Applied { queued, preview } = plan(&db, &clock, &planned, false).unwrap() else {
            panic!("applies");
        };
        assert_eq!(queued, 1);
        assert_eq!(preview.episodes[0].episode, 1.0, "next unwatched, never the whole season");
    }

    #[test]
    fn the_daily_cap_holds() {
        let db = Db::open_in_memory().unwrap();
        let clock = clock();
        let mut settings = Settings::load(&db).unwrap();
        settings.first_run_complete = true;
        settings.daily_torrent_cap = 3;
        Settings::save(&settings, &db).unwrap();

        // Five shows each wanting one episode.
        let many = MalList {
            entries: (1..=5)
                .map(|i| ListEntry {
                    mal_id: i * 10,
                    title: format!("Show {i}"),
                    english_title: None,
                    list_status: ListStatus::Watching,
                    watched_count: 0,
                    total_episodes: Some(12),
                    start_year: Some(2025),
                    auto: false,
                    episode_offset: 0,
                    names: vec![format!("Show {i}")],
                    franchise: i * 10,
                    season: 1,
                })
                .collect(),
        };
        let PlanOutcome::Applied { queued, preview } = plan(&db, &clock, &many, false).unwrap() else {
            panic!("applies");
        };
        assert_eq!(queued, 3, "the cap holds");
        assert_eq!(preview.over_cap, 2, "the rest are shown as waiting");
    }

    #[test]
    fn a_fake_list_loads_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("list.json");
        std::fs::write(&path, serde_json::to_string_pretty(&list()).unwrap()).unwrap();
        let loaded = MalList::load(&path).unwrap();
        assert_eq!(loaded.chains().len(), 1);
        assert_eq!(loaded.chains()[0].len(), 2);
        let entries = &loaded.chains()[0].entries;
        assert_eq!(entries[0].mal_id, 100);
        assert_eq!(entries[1].mal_id, 200);
    }
}
