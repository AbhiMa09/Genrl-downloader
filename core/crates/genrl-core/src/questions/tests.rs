use super::*;

fn db() -> Db {
    Db::open_in_memory().unwrap()
}

fn now() -> i64 {
    1_789_227_900
}

fn review_payload() -> Payload {
    Payload::MatchReview {
        expected: ExpectedShow {
            mal_id: 42,
            title: "Kimi no Na wa".to_string(),
            english_title: Some("Your Name".to_string()),
            year: Some(2016),
        },
        options: vec![MatchOption {
            info_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            name: "[SubsPlease] Kimi no Na wa (2016) [BD1080p]".to_string(),
            magnet: "magnet:?xt=urn:btih:aaaa".to_string(),
            group: Some("SubsPlease".to_string()),
            resolution: Some("1080p".to_string()),
            seeders: 53,
            size_bytes: 1_600_000_000,
            signal: 90.0,
            ranking: 110.0,
            title_ratio: 1.0,
            why: vec!["title matches \"kimi no na wa\" (1.00)".to_string()],
        }],
        reason: "runner-up is only 12 points behind".to_string(),
    }
}

#[test]
fn a_question_opens_gets_answered_and_stops_showing() {
    let db = db();
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, status) VALUES (42, 'Kimi no Na wa', 'movie', 'finished')",
            [],
        )?;
        Ok(())
    })
    .unwrap();
    let id = QuestionStore::ask(
        &db,
        None,
        Some(42),
        &review_payload(),
        now(),
        Some(72 * 3600),
    )
    .unwrap();

    let open = QuestionStore::open(&db, now()).unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].kind(), QuestionKind::MatchReview);
    match &open[0].payload {
        Payload::MatchReview {
            expected, options, ..
        } => {
            assert_eq!(expected.title, "Kimi no Na wa");
            assert_eq!(options.len(), 1);
            assert_eq!(options[0].group.as_deref(), Some("SubsPlease"));
        }
        other => panic!("wrong payload: {other:?}"),
    }
    assert_eq!(QuestionStore::open_count(&db, now()).unwrap(), 1);
    assert_eq!(QuestionStore::for_show(&db, 42, now()).unwrap().len(), 1);
    assert!(QuestionStore::for_show(&db, 43, now()).unwrap().is_empty());

    // Answer with the "always do this" box ticked.
    QuestionStore::answer(
        &db,
        id,
        &Answer {
            choice: Choice::UseThis {
                info_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            },
            always: true,
        },
    )
    .unwrap();

    assert!(QuestionStore::open(&db, now()).unwrap().is_empty());
    assert_eq!(QuestionStore::open_count(&db, now()).unwrap(), 0);
}

#[test]
fn unanswered_questions_expire_and_return_for_cleanup() {
    let db = db();
    // A real job id, so the question hangs off a queue row like in life.
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, status) VALUES (99, 'Show', 'tv', 'airing')",
            [],
        )?;
        c.execute(
            "INSERT INTO episodes (id, mal_id, number) VALUES (5, 99, 4)",
            [],
        )?;
        c.execute(
            "INSERT INTO jobs (id, episode_id, kind, state, created_at, updated_at)
             VALUES (7, 5, 'search', 'needs_input', 0, 0)",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    let payload = Payload::LowSeeds {
        seeders: 4,
        release: "[Group] Show - 05 (1080p)".to_string(),
        magnet: "magnet:?xt=urn:btih:bbbb".to_string(),
        auto_list: false,
    };
    let id = QuestionStore::ask(&db, Some(7), Some(99), &payload, now(), Some(3600)).unwrap();

    assert!(
        QuestionStore::expire(&db, now()).unwrap().is_empty(),
        "not yet"
    );
    let expired = QuestionStore::expire(&db, now() + 3601).unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].id, id);
    assert_eq!(expired[0].job_id, Some(7));
    assert_eq!(
        expired[0].mal_id,
        Some(99),
        "the caller returns the episode to missing"
    );

    // Expiring marks it answered, so it stops showing in the Inbox.
    assert!(QuestionStore::open(&db, now() + 3601).unwrap().is_empty());
    // And deleting is the undo behind "never ask me this again".
    assert!(QuestionStore::delete(&db, id).unwrap());
    assert!(!QuestionStore::delete(&db, id).unwrap());
}

#[test]
fn payloads_round_trip_through_the_database() {
    let db = db();
    let payloads = vec![
        review_payload(),
        Payload::FileExists {
            episode: 5.0,
            existing: "/library/Show - S01E05.mkv".to_string(),
            candidate: "[Group] Show - 05 (1080p) v2".to_string(),
        },
        Payload::BatchOnly {
            release: "[Judas] Show [Batch]".to_string(),
            seeders: 12,
        },
        Payload::DiskSpace {
            free_gb: 3,
            floor_gb: 20,
        },
        Payload::AllFailed {
            candidates_tried: 3,
        },
        Payload::EntryChanged {
            mal_id: 5,
            title: "Show".to_string(),
            detail: "MAL merged the entry into another id".to_string(),
        },
    ];
    for (index, payload) in payloads.iter().enumerate() {
        let id = QuestionStore::ask(&db, None, None, payload, now(), None).unwrap();
        let open = QuestionStore::open(&db, now()).unwrap();
        let stored = open.iter().find(|q| q.id == id).expect("just asked");
        assert_eq!(
            &stored.payload, payload,
            "payload {index} survived the round trip"
        );
    }
}

#[test]
fn kinds_are_listable_for_the_inbox_filter_tabs() {
    let db = db();
    QuestionStore::ask(
        &db,
        None,
        None,
        &Payload::LowSeeds {
            seeders: 4,
            release: "r".to_string(),
            magnet: "m".to_string(),
            auto_list: true,
        },
        now(),
        None,
    )
    .unwrap();
    assert_eq!(
        QuestionStore::of_kind(&db, QuestionKind::LowSeeds, now())
            .unwrap()
            .len(),
        1
    );
    assert!(QuestionStore::of_kind(&db, QuestionKind::FileExists, now())
        .unwrap()
        .is_empty());
}
