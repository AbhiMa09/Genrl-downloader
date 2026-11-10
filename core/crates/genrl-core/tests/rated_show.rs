use genrl_core::clock::FakeClock;
use genrl_core::db::Db;
use genrl_core::models::QuestionKind;
use genrl_core::planner::{plan, MalList};
use genrl_core::questions::{Answer, Choice, Payload, QuestionStore};
use genrl_core::season_map;
use genrl_core::worker::apply_answer;

const NOW: i64 = 1_789_227_900;

#[test]
fn auto_plan_rated_partially_watched_show_opens_question_and_is_idempotent() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, auto, metadata_status)
             VALUES (235, 'Meitantei Conan', 0, 'airing', 1, 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at)
             VALUES (235, 'watching', 1020, 8, ?1)",
            [NOW],
        )?;
        c.execute(
            "INSERT OR REPLACE INTO season_map (mal_id, season_no, first_ep, last_ep, source, created_at)
             VALUES (235, 1, 1, 1093, 'bundled', ?1)",
            [NOW],
        )?;


        season_map::ensure_episodes_exist(c, 235)?;
        Ok(())
    })
    .unwrap();


    let list = MalList::load_from_db(&db).unwrap();
    let outcome = plan(&db, &clock, &list, true).unwrap();

    // Must not queue any episodes directly
    match outcome {
        genrl_core::planner::PlanOutcome::Applied { queued, preview } => {
            assert_eq!(queued, 0, "must not queue any episodes for rated partially-watched show");
            assert!(preview.episodes.is_empty());
        }
        genrl_core::planner::PlanOutcome::Preview(p) => {
            assert!(p.episodes.is_empty());
        }
    }

    // Must open exactly 1 RatedShow question
    let open = QuestionStore::open(&db, NOW).unwrap();
    assert_eq!(open.len(), 1, "exactly one inbox question must be opened");
    assert_eq!(open[0].kind(), QuestionKind::RatedShow);
    match &open[0].payload {
        Payload::RatedShow {
            mal_id,
            score,
            watched_count,
            total_episodes,
            ..
        } => {
            assert_eq!(*mal_id, 235);
            assert_eq!(*score, 8);
            assert_eq!(*watched_count, 1020);
            assert_eq!(*total_episodes, Some(1093));
        }
        other => panic!("expected RatedShow payload, got {other:?}"),
    }

    // Idempotent: running plan again does not duplicate question
    let _ = plan(&db, &clock, &list, true).unwrap();
    let open2 = QuestionStore::open(&db, NOW).unwrap();
    assert_eq!(open2.len(), 1, "plan rerun must be idempotent and not duplicate open question");
}

#[test]
fn auto_plan_rated_fully_watched_show_skips_silently() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, auto, metadata_status)
             VALUES (50, 'Fullmetal Alchemist', 64, 'finished', 1, 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at)
             VALUES (50, 'completed', 64, 10, ?1)",
            [NOW],
        )?;
        Ok(())
    })
    .unwrap();

    let list = MalList::load_from_db(&db).unwrap();
    let outcome = plan(&db, &clock, &list, true).unwrap();

    match outcome {
        genrl_core::planner::PlanOutcome::Applied { queued, preview } => {
            assert_eq!(queued, 0);
            assert!(preview.episodes.is_empty());
        }
        genrl_core::planner::PlanOutcome::Preview(p) => {
            assert!(p.episodes.is_empty());
        }
    }

    // Must skip silently without opening any questions
    let open = QuestionStore::open(&db, NOW).unwrap();
    assert!(open.is_empty(), "fully watched rated show must not open questions");
}

#[test]
fn auto_plan_unrated_show_plans_normally() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, auto, metadata_status)
             VALUES (60, 'New Airing Show', 12, 'airing', 1, 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at)
             VALUES (60, 'watching', 2, 0, ?1)",
            [NOW],
        )?;
        c.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('first_run_complete', 'true')",
            [],
        )?;
        Ok(())

    })
    .unwrap();

    let list = MalList::load_from_db(&db).unwrap();
    let outcome = plan(&db, &clock, &list, true).unwrap();

    match outcome {
        genrl_core::planner::PlanOutcome::Applied { queued, preview } => {
            assert_eq!(queued, 1, "unrated show queues next episode normally");
            assert_eq!(preview.episodes.len(), 1);
            assert_eq!(preview.episodes[0].episode, 3.0);
        }
        other => panic!("expected Applied outcome, got {other:?}"),
    }

    let open = QuestionStore::open(&db, NOW).unwrap();
    assert!(open.is_empty(), "unrated show must not open RatedShow question");
}

#[test]
fn answering_rated_show_question_with_resume_skips_watched_episodes() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, auto, metadata_status)
             VALUES (235, 'Meitantei Conan', 0, 'airing', 1, 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at)
             VALUES (235, 'watching', 1020, 8, ?1)",
            [NOW],
        )?;
        c.execute(
            "INSERT OR REPLACE INTO season_map (mal_id, season_no, first_ep, last_ep, source, created_at)
             VALUES (235, 1, 1, 1093, 'bundled', ?1)",
            [NOW],
        )?;


        season_map::ensure_episodes_exist(c, 235)?;
        Ok(())
    })
    .unwrap();

    let list = MalList::load_from_db(&db).unwrap();
    let _ = plan(&db, &clock, &list, true).unwrap();
    let open = QuestionStore::open(&db, NOW).unwrap();
    let q = &open[0];
    let question = QuestionStore::get(&db, q.id).unwrap().unwrap();

    // User chooses: Resume
    let ans = Answer {
        choice: Choice::Resume,
        always: false,
    };
    apply_answer(&db, &clock, &question, &ans).unwrap();
    QuestionStore::answer(&db, q.id, &ans).unwrap();

    // Check that episodes 1..=1020 were marked 'done'
    let done_count: i64 = db.with(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM episodes WHERE mal_id = 235 AND state = 'done' AND number <= 1020",
            [],
            |r| r.get(0),
        )?)
    }).unwrap();
    assert_eq!(done_count, 1020, "episodes 1..=1020 must be marked done");

    // Missing remaining episodes are 1021..=1093 (73 episodes)
    let missing_count: i64 = db.with(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM episodes WHERE mal_id = 235 AND state = 'missing'",
            [],
            |r| r.get(0),
        )?)
    }).unwrap();
    assert_eq!(missing_count, 73, "remaining missing count should be 73");
}

#[test]
fn answering_rated_show_question_with_skip_sets_auto_zero() {
    let clock = FakeClock::new(NOW);
    let db = Db::open_in_memory().unwrap();

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, total_episodes, status, auto, metadata_status)
             VALUES (235, 'Meitantei Conan', 0, 'airing', 1, 'ready')",
            [],
        )?;
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at)
             VALUES (235, 'watching', 1020, 8, ?1)",
            [NOW],
        )?;
        c.execute(
            "INSERT OR REPLACE INTO season_map (mal_id, season_no, first_ep, last_ep, source, created_at)
             VALUES (235, 1, 1, 1093, 'bundled', ?1)",
            [NOW],
        )?;


        season_map::ensure_episodes_exist(c, 235)?;
        Ok(())
    })
    .unwrap();

    let list = MalList::load_from_db(&db).unwrap();
    let _ = plan(&db, &clock, &list, true).unwrap();
    let open = QuestionStore::open(&db, NOW).unwrap();
    let q = &open[0];
    let question = QuestionStore::get(&db, q.id).unwrap().unwrap();

    // User chooses: SkipShow
    let ans = Answer {
        choice: Choice::SkipShow,
        always: false,
    };
    apply_answer(&db, &clock, &question, &ans).unwrap();
    QuestionStore::answer(&db, q.id, &ans).unwrap();


    // Check shows.auto is now 0
    let auto_val: i64 = db.with(|c| {
        Ok(c.query_row("SELECT auto FROM shows WHERE mal_id = 235", [], |r| r.get(0))?)
    }).unwrap();
    assert_eq!(auto_val, 0, "shows.auto must be set to 0");

    // Subsequent plan run with updated MalList skips completely
    let list2 = MalList::load_from_db(&db).unwrap();
    let outcome = plan(&db, &clock, &list2, true).unwrap();
    match outcome {
        genrl_core::planner::PlanOutcome::Applied { queued, preview } => {
            assert_eq!(queued, 0);
            assert!(preview.episodes.is_empty());
        }
        genrl_core::planner::PlanOutcome::Preview(p) => {
            assert!(p.episodes.is_empty());
        }
    }
}
