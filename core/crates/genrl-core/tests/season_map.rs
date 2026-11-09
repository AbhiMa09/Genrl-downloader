use genrl_core::clock::SystemClock;
use genrl_core::db::Db;
use genrl_core::filename::parse_release;
use genrl_core::models::{JobState, ShowType};
use genrl_core::questions::{Answer, Choice, Payload, QuestionStore};
use genrl_core::rules::Rules;
use genrl_core::score::{evaluate, Candidate, Rejection, Target};
use genrl_core::season_map::{all_for_show, latest_season, lookup, upsert_user, SeasonMapSource};
use genrl_core::settings::Settings;

fn mock_candidate(raw: &str, seeders: u32) -> Candidate {
    let release = parse_release(raw);
    Candidate {
        release,
        info_hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
        magnet: "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567".to_string(),
        seeders,
        leechers: 5,
        trusted: true,
        remake: false,
        size_bytes: 500 * 1024 * 1024,
        published_at: 1_700_000_000,
        uploader: None,
        show_type: Some(ShowType::Tv),
        category_id: Some("1_2".to_string()),
    }
}

#[test]
fn test_season_map_bundled_seeds_and_user_override() {
    let db = Db::open_in_memory().expect("in-memory db");

    // Bundled seeds for Detective Conan (MAL 235)
    let s33 = db
        .with(|c| lookup(c, 235, 33))
        .expect("db lookup")
        .expect("season 33 exists in bundled seeds");
    assert_eq!(s33.mal_id, 235);
    assert_eq!(s33.season_no, 33);
    assert_eq!(s33.first_ep, 1050);
    assert_eq!(s33.last_ep, 1093);
    assert_eq!(s33.source, SeasonMapSource::Bundled);

    let latest = db
        .with(|c| latest_season(c, 235))
        .expect("db latest")
        .expect("latest exists");
    assert_eq!(latest.season_no, 33);

    // User override wins
    db.with(|c| {
        upsert_user(c, 235, 33, 1050, 1100, Some("Extended S33"), Some(2021), 1_700_000_000)
    })
    .expect("upsert user");

    let s33_updated = db
        .with(|c| lookup(c, 235, 33))
        .expect("db lookup")
        .expect("season 33 exists");
    assert_eq!(s33_updated.last_ep, 1100);
    assert_eq!(s33_updated.source, SeasonMapSource::User);
    assert_eq!(s33_updated.title.as_deref(), Some("Extended S33"));
}

#[test]
fn test_four_digit_episode_parsing() {
    let r1 = parse_release("[SubsPlease] Detective Conan - 1125 (1080p) [ABCD1234].mkv");
    assert_eq!(r1.episode, Some(1125.0));
    assert!(!r1.is_batch);

    let r2 = parse_release("[Group] Detective Conan - 1050-1093 [Batch] [1080p]");
    assert!(r2.is_batch);
    assert_eq!(r2.episode_range, Some((1050.0, 1093.0)));

    let r3 = parse_release("[Group] Detective Conan Season 33 - 05 [1080p]");
    assert_eq!(r3.season, Some(33));
    assert_eq!(r3.episode, Some(5.0));
}

#[test]
fn test_evaluate_priority_contract_mapped_relative_episode() {
    let db = Db::open_in_memory().expect("in-memory db");
    let maps = db.with(|c| all_for_show(c, 235)).expect("load conan maps");

    let names = vec!["detective conan".to_string(), "meitantei conan".to_string()];
    let other_names: Vec<String> = vec![];

    // Target Conan wanted ep 1054
    let target = Target {
        names: &names,
        other_names: &other_names,
        wanted_episode: Some(1054.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: None,
        season_maps: &maps,
        missing_episodes: &[],
        owned_episodes: &[],
    };

    // Candidate specifies Season 33 episode 5 -> 1050 + 5 - 1 = 1054
    let cand = mock_candidate("[SubsPlease] Detective Conan Season 33 - 05 (1080p)", 25);
    let scored = evaluate(&cand, &target, &Rules::default(), &Settings::default(), 1_700_000_000)
        .expect("evaluate should pass");
    assert!(scored.signal >= 80.0, "signal should match: {}", scored.signal);

    // Wrong season-local episode (e.g. S33 ep 10 -> absolute 1059 vs wanted 1054)
    let cand_wrong = mock_candidate("[SubsPlease] Detective Conan Season 33 - 10 (1080p)", 25);
    let err = evaluate(
        &cand_wrong,
        &target,
        &Rules::default(),
        &Settings::default(),
        1_700_000_000,
    )
    .unwrap_err();
    assert_eq!(
        err,
        Rejection::WrongEpisode {
            wanted: 1054.0,
            found: 1059.0
        }
    );
}

#[test]
fn test_evaluate_priority_contract_mapped_batch() {
    let db = Db::open_in_memory().expect("in-memory db");
    let maps = db.with(|c| all_for_show(c, 235)).expect("load conan maps");

    let names = vec!["detective conan".to_string()];
    let other_names: Vec<String> = vec![];

    let target = Target {
        names: &names,
        other_names: &other_names,
        wanted_episode: Some(1054.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: None,
        season_maps: &maps,
        missing_episodes: &[],
        owned_episodes: &[],
    };

    let mut batch_cand = mock_candidate(
        "[Group] Detective Conan Season 33 (2023) [Batch] [1080p]",
        30,
    );
    batch_cand.size_bytes = 10 * 1024 * 1024 * 1024; // 10 GB across 44 eps
    let scored = evaluate(
        &batch_cand,
        &target,
        &Rules::default(),
        &Settings::default(),
        1_700_000_000,
    )
    .expect("batch covering ep 1054 must pass");
    assert_eq!(scored.signal, 60.0); // 50 title + 10 type (batch while airing earns no batch points)
}

#[test]
fn test_evaluate_priority_contract_no_season_marker_absolute() {
    let db = Db::open_in_memory().expect("in-memory db");
    let maps = db.with(|c| all_for_show(c, 235)).expect("load conan maps");

    let names = vec!["detective conan".to_string()];
    let other_names: Vec<String> = vec![];

    let target = Target {
        names: &names,
        other_names: &other_names,
        wanted_episode: Some(1054.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: None,
        season_maps: &maps,
        missing_episodes: &[],
        owned_episodes: &[],
    };

    // No season marker in release -> absolute episode 1054
    let cand = mock_candidate("[SubsPlease] Detective Conan - 1054 (1080p)", 40);
    let scored = evaluate(&cand, &target, &Rules::default(), &Settings::default(), 1_700_000_000)
        .expect("absolute candidate without season marker must evaluate");
    assert_eq!(scored.signal, 90.0); // 50 title + 30 episode + 10 type
}

#[test]
fn test_evaluate_priority_contract_unmapped_season_rejects() {
    let db = Db::open_in_memory().expect("in-memory db");
    let maps = db.with(|c| all_for_show(c, 235)).expect("load conan maps");

    let names = vec!["detective conan".to_string()];
    let other_names: Vec<String> = vec![];

    let target = Target {
        names: &names,
        other_names: &other_names,
        wanted_episode: Some(1054.0),
        show_type: ShowType::Tv,
        season: 1,
        start_year: Some(1996),
        fully_aired: false,
        total_episodes: None,
        season_maps: &maps,
        missing_episodes: &[],
        owned_episodes: &[],
    };

    // Release specifies Season 40, which is unmapped
    let cand = mock_candidate("[SubsPlease] Detective Conan Season 40 - 01 (1080p)", 20);
    let err = evaluate(&cand, &target, &Rules::default(), &Settings::default(), 1_700_000_000)
        .unwrap_err();
    assert_eq!(
        err,
        Rejection::SeasonConflict {
            wanted: 1,
            found: 40
        }
    );
}

#[test]
fn test_inbox_apply_season_range_answer() {
    let db = Db::open_in_memory().expect("in-memory db");
    let clock = SystemClock;

    // Create a dummy show and job
    let mal_id = 99999;
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, season, total_episodes, type, status, metadata_status)
             VALUES (?1, 'Test Long Show', 1, 0, 'tv', 'currently_airing', 'ready')",
            [mal_id],
        )?;
        c.execute(
            "INSERT INTO episodes (mal_id, number, state) VALUES (?1, 1.0, 'missing')",
            [mal_id],
        )?;
        c.execute(
            "INSERT INTO jobs (episode_id, kind, state, created_at, updated_at)
             VALUES (1, 'search', 'needs_input', 1000, 1000)",
            [],
        )?;
        Ok(())
    })
    .expect("insert test show and job");

    let question_id = QuestionStore::ask(
        &db,
        Some(1),
        Some(mal_id),
        &Payload::SeasonMap {
            mal_id,
            season_no: 5,
            release_title: "[Group] Test Long Show Season 5 [Batch]".to_string(),
            suggested_first: 101,
            suggested_last: 125,
            reason: "Unmapped season 5".to_string(),
        },
        1000,
        None,
    )
    .expect("ask question");

    let q = QuestionStore::get(&db, question_id).expect("get q").expect("exists");
    let ans = Answer {
        choice: Choice::SeasonRange {
            first_ep: 101,
            last_ep: 125,
        },
        always: false,
    };

    let next_state = genrl_core::worker::apply_answer(&db, &clock, &q, &ans)
        .expect("apply answer")
        .expect("next state");
    assert_eq!(next_state, JobState::Search);

    // Verify season_map row was created as user source
    let entry = db
        .with(|c| lookup(c, mal_id, 5))
        .expect("lookup")
        .expect("entry created");
    assert_eq!(entry.first_ep, 101);
    assert_eq!(entry.last_ep, 125);
    assert_eq!(entry.source, SeasonMapSource::User);

    // Verify episodes 101..125 exist in episodes table
    let count: i64 = db
        .with(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM episodes WHERE mal_id = ?1 AND number >= 101 AND number <= 125",
                [mal_id],
                |r| r.get(0),
            )?;
            Ok(n)
        })
        .expect("count episodes");
    assert_eq!(count, 25);
}

#[test]
fn test_planner_apply_total_episodes_zero_with_season_map() {
    let db = Db::open_in_memory().expect("in-memory db");
    let clock = SystemClock;

    // Seed Detective Conan show entry with total_episodes = 0 and status = 'airing'
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, season, total_episodes, type, status, metadata_status)
             VALUES (235, 'Detective Conan', 1, 0, 'tv', 'airing', 'ready')",
            [],
        )?;
        Ok(())
    })
    .expect("insert conan");

    // Plan single for show without target_episode
    let queued = genrl_core::planner::plan_single(&db, &clock, 235, None)
        .expect("plan_single must succeed");
    // Queues 1 pack search job covering the season
    assert_eq!(queued, 1, "one pack search job for season 33");

    // Verify all 44 episodes for the latest season (Season 33: 1050..1093) exist in episodes table
    let ep_count: i64 = db
        .with(|c| {
            let n: i64 = c.query_row(
                "SELECT COUNT(*) FROM episodes
                 WHERE mal_id = 235 AND number >= 1050 AND number <= 1093",
                [],
                |r| r.get(0),
            )?;
            Ok(n)
        })
        .expect("query episode count");
    assert_eq!(ep_count, 44, "All 44 episodes of latest season 33 must exist in episodes");

    // Verify the queued search job is for episode 1050 (start of season 33, not stalled on ep 1)
    let job_ep_number: f64 = db
        .with(|c| {
            let n: f64 = c.query_row(
                "SELECT e.number FROM jobs j JOIN episodes e ON e.id = j.episode_id
                 WHERE e.mal_id = 235",
                [],
                |r| r.get(0),
            )?;
            Ok(n)
        })
        .expect("query job episode number");
    assert_eq!(job_ep_number, 1050.0);
}
