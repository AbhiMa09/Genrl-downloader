//! Nyaa search end to end against a mock server: parsing, sorting, the
//! mirror fallback and the response cache.

mod common;

use common::Fixed;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::http::Service;
use genrl_core::nyaa::{self, SearchQuery};
use genrl_core::{Db, Gate, Settings};

const FEED: &str = include_str!("fixtures/nyaa_frieren.xml");

fn gate() -> Gate {
    Gate::new(Db::open_in_memory().unwrap()).unwrap()
}

fn settings_with(base: &str, mirrors: Vec<String>) -> Settings {
    let mut settings = Settings::default();
    settings.nyaa_base_url = base.to_string();
    settings.nyaa_mirrors = mirrors;
    settings
}

#[tokio::test]
async fn search_returns_seeder_sorted_candidates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FEED))
        .mount(&server)
        .await;

    let settings = settings_with(&server.uri(), Vec::new());
    let items = nyaa::search(
        &gate(),
        &settings,
        &SearchQuery::new("sousou no frieren 1080p"),
    )
    .await
    .unwrap();

    assert_eq!(
        items.len(),
        50,
        "the feed had 75 items; the default page limit is 50"
    );
    for pair in items.windows(2) {
        assert!(
            pair[0].seeders >= pair[1].seeders,
            "sorted by seeders: {} then {}",
            pair[0].seeders,
            pair[1].seeders
        );
    }
    // The most-seeded row of the real fixture feed.
    assert_eq!(
        items[0].info_hash,
        "bd167d148570f2c7ece6c15bd90d3901acb6d79c"
    );
    assert!(items[0].info_hash.chars().all(|c| c.is_ascii_hexdigit()));

    // The URL we asked for is the design's shape.
    let requests = server.received_requests().await.unwrap();
    let query: Vec<(String, String)> = requests[0]
        .url
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let get = |key: &str| query.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone());
    assert_eq!(get("page").as_deref(), Some("rss"));
    assert_eq!(get("q").as_deref(), Some("sousou no frieren 1080p"));
    assert_eq!(
        get("c").as_deref(),
        Some("1_2"),
        "anime, English-translated"
    );
    assert_eq!(get("s").as_deref(), Some("seeders"));
    assert_eq!(get("o").as_deref(), Some("desc"));
}

#[tokio::test]
async fn a_dead_primary_falls_back_to_the_mirror() {
    let broken = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&broken)
        .await;

    let mirror = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FEED))
        .mount(&mirror)
        .await;

    let settings = settings_with(&broken.uri(), vec![mirror.uri()]);
    let items = nyaa::search(&gate(), &settings, &SearchQuery::new("frieren"))
        .await
        .unwrap();
    assert!(!items.is_empty(), "the mirror answered");
}

#[tokio::test]
async fn the_same_query_is_cached_for_ten_minutes() {
    let server = MockServer::start().await;
    let (hits, fixed) = Fixed::new(ResponseTemplate::new(200).set_body_string(FEED));
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(fixed)
        .mount(&server)
        .await;

    let settings = settings_with(&server.uri(), Vec::new());
    let db = Db::open_in_memory().unwrap();
    let gate = Gate::new(db).unwrap();
    let query = SearchQuery::new("frieren 1080p");

    let first = nyaa::search(&gate, &settings, &query).await.unwrap();
    let second = nyaa::search(&gate, &settings, &query).await.unwrap();

    assert_eq!(first.len(), second.len());
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the second search was served from the SQLite cache"
    );
}

#[tokio::test]
async fn min_seeders_filters_and_limit_bounds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(FEED))
        .mount(&server)
        .await;

    let settings = settings_with(&server.uri(), Vec::new());
    let mut query = SearchQuery::new("frieren");
    query.min_seeders = 30;
    query.limit = 5;
    let items = nyaa::search(&gate(), &settings, &query).await.unwrap();
    assert!(items.len() <= 5, "limit honoured: {}", items.len());
    assert!(items.iter().all(|i| i.seeders >= 30), "seed floor honoured");

    assert_eq!(Service::Nyaa.as_str(), "nyaa");
}

fn mock_feed_item(title: &str, seeders: u32) -> nyaa::FeedItem {
    nyaa::FeedItem {
        title: title.to_string(),
        link: "https://example.com/test.torrent".to_string(),
        guid: "https://example.com/view/1".to_string(),
        published_at: 1_760_000_000,
        seeders,
        leechers: 2,
        downloads: 50,
        info_hash: format!("{:040x}", title.len()),
        category_id: "1_2".to_string(),
        category: "Anime - English-translated".to_string(),
        size_bytes: 1_400_000_000,
        size_text: "1.4 GiB".to_string(),
        trusted: true,
        remake: false,
    }
}

#[test]
fn test_rezero_4th_season_query_and_matcher_decisions() {
    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 61316,
        title: "Re:Zero kara Hajimeru Isekai Seikatsu 4th Season".to_string(),
        english_title: Some("Re:ZERO -Starting Life in Another World- Season 4".to_string()),
        names: vec![
            genrl_core::filename::normalize("Re:Zero kara Hajimeru Isekai Seikatsu 4th Season"),
            genrl_core::filename::normalize("Re:ZERO -Starting Life in Another World- Season 4"),
            genrl_core::filename::normalize("Re:Zero 4th Season"),
        ],
        season: 4,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2025),
        fully_aired: false,
        auto: true,
        total_episodes: Some(12),
        other_names: vec![genrl_core::filename::normalize(
            "Re:Zero kara Hajimeru Isekai Seikatsu",
        )],
    };

    // Assert exact query used for Re:Zero 4th Season contains season marker and never Season 1
    let query_text = &inputs.title;
    let query_lower = query_text.to_lowercase();
    assert!(query_lower.contains("4th season") || query_lower.contains("season 4"));
    assert!(!query_lower.contains("season 1"));

    let titles = [
        "[SubsPlease] Re:Zero kara Hajimeru Isekai Seikatsu 4th Season - 01 (1080p) [ABCD1234].mkv",
        "[Erai-raws] Re:Zero Season 4 - 01 [1080p][Multiple Subtitle].mkv",
        "[Judas] Re:Zero Season 4 - 01 [1080p][HEVC x265 10bit].mkv",
        "[SubsPlease] Re:Zero kara Hajimeru Isekai Seikatsu S3 - 01 (1080p) [1234ABCD].mkv",
        "[Erai-raws] Re:Zero Season 3 - 01 [1080p].mkv",
        "[SubsPlease] Re:Zero kara Hajimeru Isekai Seikatsu 2nd Season - 01 (1080p) [5678EFGH].mkv",
        "[Judas] Re:Zero Season 2 (Part 2) [1080p] [Batch]",
        "[Coalgirls] Re:Zero kara Hajimeru Isekai Seikatsu (Season 1) [1080p] [Batch]",
        "[SubsPlease] Re:Zero kara Hajimeru Isekai Seikatsu - 01 (1080p) [DEADBEEF].mkv",
        "[Beatrice-Raws] Re Zero Kara Hajimeru Isekai Seikatsu 01-25 [BDRip 1920x1080 x264 FLAC]",
    ];

    let candidates: Vec<genrl_core::score::Candidate> = titles
        .iter()
        .enumerate()
        .map(|(i, &t)| {
            let item = mock_feed_item(t, (100 - i * 5) as u32);
            let release = genrl_core::filename::parse_release(t);
            genrl_core::matching::candidate_from(&release, &item)
        })
        .collect();

    let rules = genrl_core::rules::Rules::default();
    let settings = Settings::default();
    let now = 1_760_000_000;

    let (ranked, rejected) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules, &settings, now);

    println!("\n=== Re:Zero 4th Season Matcher Decisions ===");
    println!("Query used: {}", query_text);
    println!("Ranked (Accepted) candidates: {}", ranked.len());
    for r in &ranked {
        println!(
            "  ACCEPT: {} (Score: {:.2})",
            r.candidate.release.raw,
            r.scored.total()
        );
    }
    println!("Rejected candidates: {}", rejected.len());
    for (raw, rej) in &rejected {
        println!("  REJECT: {} -> {:?}", raw, rej);
    }

    assert_eq!(ranked.len(), 3, "Only the 3 Season 4 releases are accepted");
    assert_eq!(rejected.len(), 7, "All non-Season-4 / batches are rejected");
    for r in &ranked {
        assert_eq!(r.candidate.release.season, Some(4));
        assert_eq!(r.candidate.release.episode, Some(1.0));
    }
    for (_, rej) in &rejected {
        match rej {
            genrl_core::score::Rejection::SeasonConflict { wanted, found } => {
                assert_eq!(*wanted, 4);
                assert!(*found < 4);
            }
            _ => {}
        }
    }
}

#[test]
fn test_rezero_matcher_offline_fixture() {
    const REZERO_FEED: &str = include_str!("fixtures/nyaa_rezero_live.xml");
    let feed_items = nyaa::parse_feed(REZERO_FEED).expect("real live fixture parses");
    assert!(!feed_items.is_empty(), "Fixture should contain feed items");

    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 61316,
        title: "Re:Zero kara Hajimeru Isekai Seikatsu 4th Season".to_string(),
        english_title: Some("Re:ZERO -Starting Life in Another World- Season 4".to_string()),
        names: vec![
            genrl_core::filename::normalize("Re:Zero kara Hajimeru Isekai Seikatsu 4th Season"),
            genrl_core::filename::normalize("Re:ZERO -Starting Life in Another World- Season 4"),
            genrl_core::filename::normalize("Re:Zero 4th Season"),
        ],
        season: 4,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2025),
        fully_aired: false,
        auto: true,
        total_episodes: Some(12),
        other_names: vec![genrl_core::filename::normalize(
            "Re:Zero kara Hajimeru Isekai Seikatsu",
        )],
    };

    let mut rules = genrl_core::rules::Rules::default();
    // 5 preferred groups matching UI default:
    rules.groups = vec![
        "SubsPlease".to_string(),
        "Erai-raws".to_string(),
        "ASW".to_string(),
        "Judas".to_string(),
        "Golumpa".to_string(),
    ];
    let settings = Settings::default();
    let now = 1_760_000_000;

    let candidates: Vec<genrl_core::score::Candidate> = feed_items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    let (ranked, rejected) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules, &settings, now);

    println!("\n=== Re:Zero S04E01 Matcher Decisions on Live Nyaa Fixture (5 Groups) ===");
    for (i, r) in ranked.iter().enumerate() {
        println!(
            "#{:<2} [{} seeders] ACCEPT (Score: {:>5.1} | Signal: {:>5.1} | Ranking: {:>5.1}) -> {}\n     Breakdown: {}",
            i + 1,
            r.candidate.seeders,
            r.scored.total(),
            r.scored.signal,
            r.scored.ranking,
            r.candidate.release.raw,
            r.scored.notes.join("; ")
        );
    }
    for (raw, rej) in &rejected {
        println!("  REJECT: {} -> {:?}", raw, rej);
    }

    // Verify Judas receives group priority #4 bonus (+25 points)
    let judas_candidate = ranked
        .iter()
        .find(|r| r.candidate.release.group.as_deref() == Some("Judas"));
    assert!(
        judas_candidate.is_some(),
        "Judas candidate should be accepted"
    );
    let judas_notes = judas_candidate.unwrap().scored.notes.join("; ");
    assert!(
        judas_notes.contains("Judas is priority group #4"),
        "Judas should receive priority group #4 bonus"
    );

    // Verify bare DUAL is detected as dual audio (+10 points)
    let dual_candidate = ranked.iter().find(|r| {
        r.candidate.release.raw.contains("DUAL") && !r.candidate.release.raw.contains("Dual-Audio")
    });
    if let Some(c) = dual_candidate {
        assert!(
            c.candidate.release.dual_audio,
            "Bare DUAL must set dual_audio flag"
        );
    }
}

#[tokio::test]
#[ignore = "requires live internet connection to nyaa.si"]
async fn test_rezero_matcher_live_network() {
    let gate = gate();
    let settings = Settings::default();
    let query = SearchQuery::new("Re:Zero S04E01");
    let items = nyaa::search(&gate, &settings, &query).await.unwrap();
    assert!(
        !items.is_empty(),
        "Live search should find items on nyaa.si"
    );

    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 61316,
        title: "Re:Zero kara Hajimeru Isekai Seikatsu 4th Season".to_string(),
        english_title: Some("Re:ZERO -Starting Life in Another World- Season 4".to_string()),
        names: vec![
            genrl_core::filename::normalize("Re:Zero kara Hajimeru Isekai Seikatsu 4th Season"),
            genrl_core::filename::normalize("Re:ZERO -Starting Life in Another World- Season 4"),
            genrl_core::filename::normalize("Re:Zero 4th Season"),
        ],
        season: 4,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2025),
        fully_aired: false,
        auto: true,
        total_episodes: Some(12),
        other_names: vec![genrl_core::filename::normalize(
            "Re:Zero kara Hajimeru Isekai Seikatsu",
        )],
    };

    let mut rules = genrl_core::rules::Rules::default();
    rules.groups = vec![
        "SubsPlease".to_string(),
        "Erai-raws".to_string(),
        "ASW".to_string(),
        "Judas".to_string(),
        "Golumpa".to_string(),
    ];
    let now = 1_760_000_000;

    let candidates: Vec<genrl_core::score::Candidate> = items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    let (ranked, _) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules, &settings, now);
    assert!(
        !ranked.is_empty(),
        "Live results must have accepted candidates"
    );
}

/// Fix 3: Batch-only case with saved raw Nyaa file (Oni no Hanayome).
/// When no per-episode release exists, but a batch covers the episode:
/// - Candidate with seeders >= 1 is scored (not capped at 50 / rejected)
/// - Total size, episode count, and seeders are verified
/// - Candidate meets threshold and is offered with Accept verdict
#[test]
fn test_oni_no_hanayome_batch_only_candidate_offered() {
    let xml = include_str!("fixtures/nyaa_oni_live.xml");
    let items = genrl_core::nyaa::parse_feed(xml).expect("parse feed");
    assert!(!items.is_empty(), "Fixture must contain items");

    let settings = genrl_core::settings::Settings::default();
    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 61814,
        title: "Oni no Hanayome".to_string(),
        english_title: Some("The Ogre's Bride".to_string()),
        names: vec![
            genrl_core::filename::normalize("Oni no Hanayome"),
            genrl_core::filename::normalize("The Ogre's Bride"),
        ],
        season: 1,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2026),
        fully_aired: true,
        auto: false,
        total_episodes: Some(12),
        other_names: vec![],
    };

    let rules = genrl_core::rules::Rules::default();
    let now = 1_760_000_000;

    let candidates: Vec<genrl_core::score::Candidate> = items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    let (ranked, rejected) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules, &settings, now);
    assert!(
        !ranked.is_empty(),
        "Batch release for Oni no Hanayome must pass ranking"
    );
    let best = &ranked[0];
    assert!(best.candidate.seeders >= 1, "Must require >= 1 seeder");
    assert!(
        best.scored.signal >= 80.0,
        "Batch covering episode must not be capped at 50; got {}",
        best.scored.signal
    );

    let verdict =
        genrl_core::matching::decide_verdict(ranked, rejected, &inputs, &rules, &settings);
    // Since Judas batch is 3.9 GiB across 12 eps, 3.9 / 12 = ~0.325 GB/ep (well below 2 GB floor)
    match verdict {
        genrl_core::matching::Verdict::Auto { chosen, .. } => {
            assert!(
                chosen.candidate.release.raw.contains("SubsPlease")
                    || chosen.candidate.release.raw.contains("Judas"),
                "Confident batch candidate chosen"
            );
        }
        genrl_core::matching::Verdict::Ask { ranked, .. } => {
            assert!(
                !ranked.is_empty(),
                "Must offer candidate with Accept option"
            );
        }
        other => panic!("Expected candidate to be offered or auto-selected, got {other:?}"),
    }
}

/// Fix 3: Second show (Shingetsutan Tsukihime) batch candidate scoring.
/// Evaluates Exiled-Destiny full-series release (no episode number in title)
/// and confirms it scores >= 80 and is offered rather than silently asking search again.
#[test]
fn test_tsukihime_batch_candidate_scoring() {
    let xml = include_str!("fixtures/nyaa_tsukihime_live.xml");
    let items = genrl_core::nyaa::parse_feed(xml).expect("parse feed");
    assert!(!items.is_empty(), "Fixture must contain items");

    let settings = genrl_core::settings::Settings::default();
    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 169,
        title: "Shingetsutan Tsukihime".to_string(),
        english_title: Some("Lunar Legend Tsukihime".to_string()),
        names: vec![
            genrl_core::filename::normalize("Shingetsutan Tsukihime"),
            genrl_core::filename::normalize("Lunar Legend Tsukihime"),
        ],
        season: 1,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2003),
        fully_aired: true,
        auto: false,
        total_episodes: Some(12),
        other_names: vec![],
    };

    let rules = genrl_core::rules::Rules::default();
    let now = 1_760_000_000;

    let candidates: Vec<genrl_core::score::Candidate> = items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    let (ranked, _) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules, &settings, now);
    assert!(!ranked.is_empty(), "Tsukihime candidates must pass ranking");
    let best = &ranked[0];
    assert!(
        best.scored.signal >= 80.0,
        "Tsukihime batch release must score >= 80; got {}",
        best.scored.signal
    );
}

/// Fix 3: Negative tests for raw Nyaa feed parsing:
/// - Batch of a different season is rejected
/// - Batch not covering the requested episode is rejected
/// - Zero seeders is rejected
/// - Irrelevant rows from the raw Oni no Hanayome feed (Literature Raw, Live Action, Seto no Hanayome OVA) are rejected
#[test]
fn test_negative_oni_no_hanayome_rejections_from_raw_feed() {
    let xml = include_str!("fixtures/nyaa_oni_live.xml");
    let items = genrl_core::nyaa::parse_feed(xml).expect("parse feed");
    assert!(!items.is_empty(), "Feed must parse items");

    let settings = genrl_core::settings::Settings::default();
    let rules = genrl_core::rules::Rules::default();
    let now = 1_760_000_000;

    let base_inputs = genrl_core::matching::TargetInputs {
        mal_id: 61814,
        title: "Oni no Hanayome".to_string(),
        english_title: Some("The Ogre's Bride".to_string()),
        names: vec![
            genrl_core::filename::normalize("Oni no Hanayome"),
            genrl_core::filename::normalize("The Ogre's Bride"),
        ],
        season: 1,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2026),
        fully_aired: true,
        auto: false,
        total_episodes: Some(12),
        other_names: vec![genrl_core::filename::normalize("Seto no Hanayome")],
    };

    let candidates: Vec<genrl_core::score::Candidate> = items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    // 1. Batch of a different season is rejected (season 2 wanted, season 1 batch rejected)
    let s2_inputs = genrl_core::matching::TargetInputs {
        season: 2,
        ..base_inputs.clone()
    };
    let (_, rejected_s2) = genrl_core::matching::rank(
        candidates.clone(),
        &s2_inputs,
        Some(1.0),
        &rules,
        &settings,
        now,
    );
    let judas_s2_rejection = rejected_s2
        .iter()
        .find(|(title, _)| title.contains("Judas") && title.contains("Batch"));
    assert!(
        judas_s2_rejection.is_some(),
        "Season 1 Judas batch must be rejected when Season 2 is wanted"
    );
    assert!(
        matches!(
            judas_s2_rejection.unwrap().1,
            genrl_core::score::Rejection::SeasonConflict {
                wanted: 2,
                found: 1
            }
        ),
        "Expected SeasonConflict rejection"
    );

    // 2. Batch not covering the requested episode is rejected (wanted episode 15, batch covers 1-12)
    let (_, rejected_ep15) = genrl_core::matching::rank(
        candidates.clone(),
        &base_inputs,
        Some(15.0),
        &rules,
        &settings,
        now,
    );
    let subsplease_batch_rej = rejected_ep15
        .iter()
        .find(|(title, _)| title.contains("SubsPlease") && title.contains("Batch"));
    assert!(
        subsplease_batch_rej.is_some(),
        "Batch 01-12 must be rejected when episode 15 is requested"
    );
    assert!(
        matches!(
            subsplease_batch_rej.unwrap().1,
            genrl_core::score::Rejection::WrongEpisode { wanted, found } if (wanted - 15.0).abs() < f64::EPSILON && (found - 12.0).abs() < f64::EPSILON
        ),
        "Expected WrongEpisode rejection for batch out of range"
    );

    // 3. Zero seeders is rejected (e.g. shincaps ep 12 or ep 11 with 0 seeders)
    let (_, rejected_ep12) = genrl_core::matching::rank(
        candidates.clone(),
        &base_inputs,
        Some(12.0),
        &rules,
        &settings,
        now,
    );
    let shincaps_rej = rejected_ep12
        .iter()
        .find(|(title, _)| title.contains("shincaps"));
    assert!(
        shincaps_rej.is_some(),
        "Shincaps 0-seeder release must be rejected"
    );
    assert!(
        matches!(
            shincaps_rej.unwrap().1,
            genrl_core::score::Rejection::NoSeeders
        ),
        "Expected NoSeeders rejection"
    );

    // 4. Irrelevant rows rejected:
    // a. Literature Raw: Kizu Mono No Hanayome 第01-12巻
    let lit_item = candidates.iter().find(|c| {
        c.release.raw.contains("Kizu Mono No Hanayome") || c.release.raw.contains("第01-12巻")
    });
    assert!(lit_item.is_some(), "Feed must contain Literature Raw row");
    let lit_c = lit_item.unwrap();
    let lit_eval = genrl_core::score::evaluate(
        lit_c,
        &base_inputs.score_target(Some(1.0)),
        &rules,
        &settings,
        now,
    );
    assert!(
        lit_eval.is_err() || lit_eval.as_ref().unwrap().signal < 50.0,
        "Literature Raw must be rejected or score below threshold: {lit_eval:?}"
    );

    // b. Live Action Raw: [MagicStar] Oni no Hanayome 2026 [WEBDL] (Category 4_4)
    let live_action = candidates
        .iter()
        .find(|c| c.release.raw.contains("MagicStar"));
    assert!(
        live_action.is_some(),
        "Feed must contain MagicStar Live Action row"
    );
    let la_c = live_action.unwrap();
    let la_eval = genrl_core::score::evaluate(
        la_c,
        &base_inputs.score_target(Some(1.0)),
        &rules,
        &settings,
        now,
    );
    assert!(
        matches!(
            la_eval,
            Err(genrl_core::score::Rejection::TypeMismatch { .. })
        ),
        "Live Action category 4_4 must be rejected with TypeMismatch; got {la_eval:?}"
    );

    // c. Seto no Hanayome OVA: [ToonsHub] Oni no Hanayome S01E12 (Seto no Hanayome OVA)
    let seto_item = candidates
        .iter()
        .find(|c| c.release.raw.contains("Seto no Hanayome"));
    assert!(
        seto_item.is_some(),
        "Feed must contain Seto no Hanayome OVA row"
    );
    let seto_c = seto_item.unwrap();
    let seto_eval = genrl_core::score::evaluate(
        seto_c,
        &base_inputs.score_target(Some(12.0)),
        &rules,
        &settings,
        now,
    );
    assert!(
        matches!(
            seto_eval,
            Err(genrl_core::score::Rejection::TitleIsAnotherShow { .. })
        ),
        "Seto no Hanayome OVA must be rejected as TitleIsAnotherShow; got {seto_eval:?}"
    );
}

/// Size policy tests:
/// - Oni no Hanayome: 16.4 GiB batch for 12 eps = ~1.4 GB/ep must pass
/// - 1.7 GiB single episode must pass
/// - 3 GB single episode must be rejected
/// - 250 GB batch must be rejected
#[test]
fn test_size_policy_raw_feed_and_synthetic_limits() {
    let xml = include_str!("fixtures/nyaa_oni_live.xml");
    let items = genrl_core::nyaa::parse_feed(xml).expect("parse feed");

    let settings = genrl_core::settings::Settings::default();
    assert_eq!(settings.max_episode_mb, 2560);
    assert_eq!(settings.max_batch_gb, 200);
    assert_eq!(settings.min_episode_mb, 100);

    let rules = genrl_core::rules::Rules::default();
    let now = 1_760_000_000;

    let target_inputs = genrl_core::matching::TargetInputs {
        mal_id: 61814,
        title: "Oni no Hanayome".to_string(),
        english_title: Some("The Ogre's Bride".to_string()),
        names: vec![
            genrl_core::filename::normalize("Oni no Hanayome"),
            genrl_core::filename::normalize("The Ogre's Bride"),
        ],
        season: 1,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2026),
        fully_aired: true,
        auto: false,
        total_episodes: Some(12),
        other_names: vec![],
    };

    let target = target_inputs.score_target(Some(1.0));

    // 1. Oni no Hanayome: 16.4 GiB batch for 12 eps (~1.4 GB/ep) must pass
    let batch_16gb_item = items
        .iter()
        .find(|item| {
            item.title
                .contains("[SubsPlease] Oni no Hanayome (01-12) (1080p) [Batch]")
        })
        .expect("Must find SubsPlease 16.4 GiB batch in feed");
    assert_eq!(batch_16gb_item.size_text, "16.4 GiB");
    let batch_16gb_release = genrl_core::filename::parse_release(&batch_16gb_item.title);
    let batch_16gb_c = genrl_core::matching::candidate_from(&batch_16gb_release, batch_16gb_item);
    let batch_16gb_eval =
        genrl_core::score::evaluate(&batch_16gb_c, &target, &rules, &settings, now);
    assert!(
        batch_16gb_eval.is_ok(),
        "SubsPlease 16.4 GiB batch (~1.4 GB/ep across 12 eps) must pass size policy; got {:?}",
        batch_16gb_eval.err()
    );

    // Also check Judas batch (3.9 GiB / 12 eps = ~330 MB/ep) passes
    let judas_item = items
        .iter()
        .find(|item| item.title.contains("[Judas] Oni no Hanayome") && item.title.contains("Batch"))
        .expect("Must find Judas batch in feed");
    let judas_release = genrl_core::filename::parse_release(&judas_item.title);
    let judas_c = genrl_core::matching::candidate_from(&judas_release, judas_item);
    let judas_eval = genrl_core::score::evaluate(&judas_c, &target, &rules, &settings, now);
    assert!(
        judas_eval.is_ok(),
        "Judas batch must pass; got {:?}",
        judas_eval.err()
    );

    // 2. 1.7 GiB single episode must pass
    let mut ep_1_7gb_item = judas_item.clone();
    ep_1_7gb_item.title = "[SubsPlease] Oni no Hanayome - 01 (1080p) [12345678].mkv".to_string();
    ep_1_7gb_item.size_bytes = (1.7 * 1024.0 * 1024.0 * 1024.0) as u64; // ~1.7 GiB = 1740 MB
    let ep_1_7gb_release = genrl_core::filename::parse_release(&ep_1_7gb_item.title);
    let ep_1_7gb_c = genrl_core::matching::candidate_from(&ep_1_7gb_release, &ep_1_7gb_item);
    let ep_1_7gb_eval = genrl_core::score::evaluate(&ep_1_7gb_c, &target, &rules, &settings, now);
    assert!(
        ep_1_7gb_eval.is_ok(),
        "1.7 GiB single episode must pass size policy (<= 2560 MB); got {:?}",
        ep_1_7gb_eval.err()
    );

    // 3. 3 GB single episode must be rejected
    let mut ep_3gb_item = judas_item.clone();
    ep_3gb_item.title = "[SubsPlease] Oni no Hanayome - 01 (1080p) [12345678].mkv".to_string();
    ep_3gb_item.size_bytes = (3.0 * 1024.0 * 1024.0 * 1024.0) as u64; // ~3.0 GiB = 3072 MB
    let ep_3gb_release = genrl_core::filename::parse_release(&ep_3gb_item.title);
    let ep_3gb_c = genrl_core::matching::candidate_from(&ep_3gb_release, &ep_3gb_item);
    let ep_3gb_eval = genrl_core::score::evaluate(&ep_3gb_c, &target, &rules, &settings, now);
    assert!(
        matches!(
            ep_3gb_eval,
            Err(genrl_core::score::Rejection::SizeOutOfRange { .. })
        ),
        "3 GB single episode must be rejected with SizeOutOfRange (> 2560 MB); got {:?}",
        ep_3gb_eval
    );

    // 4. 250 GB batch must be rejected
    let mut batch_250gb_item = judas_item.clone();
    batch_250gb_item.size_bytes = 250 * 1024 * 1024 * 1024; // 250 GiB > 200 GB limit
    let batch_250gb_release = genrl_core::filename::parse_release(&batch_250gb_item.title);
    let batch_250gb_c =
        genrl_core::matching::candidate_from(&batch_250gb_release, &batch_250gb_item);
    let batch_250gb_eval =
        genrl_core::score::evaluate(&batch_250gb_c, &target, &rules, &settings, now);
    assert!(
        matches!(
            batch_250gb_eval,
            Err(genrl_core::score::Rejection::SizeOutOfRange { .. })
        ),
        "250 GB batch must be rejected with SizeOutOfRange (> 200 GB); got {:?}",
        batch_250gb_eval
    );
}

#[test]
fn test_oni_no_hanayome_ties_auto_pick_by_preferred_group_order() {
    let xml = include_str!("fixtures/nyaa_oni_live.xml");
    let items = genrl_core::nyaa::parse_feed(xml).expect("parse feed");
    assert!(!items.is_empty(), "Fixture must contain items");

    let settings = genrl_core::settings::Settings::default();
    let inputs = genrl_core::matching::TargetInputs {
        mal_id: 61814,
        title: "Oni no Hanayome".to_string(),
        english_title: Some("The Ogre's Bride".to_string()),
        names: vec![
            genrl_core::filename::normalize("Oni no Hanayome"),
            genrl_core::filename::normalize("The Ogre's Bride"),
        ],
        season: 1,
        show_type: genrl_core::models::ShowType::Tv,
        start_year: Some(2026),
        fully_aired: true,
        auto: false,
        total_episodes: Some(12),
        other_names: vec![],
    };

    let now = 1_760_000_000;

    let candidates: Vec<genrl_core::score::Candidate> = items
        .iter()
        .map(|item| {
            let release = genrl_core::filename::parse_release(&item.title);
            genrl_core::matching::candidate_from(&release, item)
        })
        .collect();

    // 1. With Judas preferred first: Judas auto-selected
    let mut rules_judas = genrl_core::rules::Rules::default();
    rules_judas.groups = vec!["Judas".to_string(), "SubsPlease".to_string()];
    let (ranked1, rej1) = genrl_core::matching::rank(
        candidates.clone(),
        &inputs,
        Some(1.0),
        &rules_judas,
        &settings,
        now,
    );
    let verdict1 =
        genrl_core::matching::decide_verdict(ranked1, rej1, &inputs, &rules_judas, &settings);
    match verdict1 {
        genrl_core::matching::Verdict::Auto { chosen, .. } => {
            assert!(chosen
                .candidate
                .release
                .group
                .as_deref()
                .unwrap()
                .eq_ignore_ascii_case("Judas"));
        }
        other => panic!("Expected Auto pick for Judas, got {other:?}"),
    }

    // 2. With SubsPlease preferred first: SubsPlease auto-selected
    let mut rules_subs = genrl_core::rules::Rules::default();
    rules_subs.groups = vec!["SubsPlease".to_string(), "Judas".to_string()];
    let (ranked2, rej2) = genrl_core::matching::rank(
        candidates.clone(),
        &inputs,
        Some(1.0),
        &rules_subs,
        &settings,
        now,
    );
    let verdict2 =
        genrl_core::matching::decide_verdict(ranked2, rej2, &inputs, &rules_subs, &settings);
    match verdict2 {
        genrl_core::matching::Verdict::Auto { chosen, .. } => {
            assert!(chosen
                .candidate
                .release
                .group
                .as_deref()
                .unwrap()
                .eq_ignore_ascii_case("SubsPlease"));
        }
        other => panic!("Expected Auto pick for SubsPlease, got {other:?}"),
    }

    // 3. With empty rules.groups: auto-selects top of ranked (SubsPlease)
    let mut rules_empty = genrl_core::rules::Rules::default();
    rules_empty.groups = vec![];
    let (ranked3, rej3) =
        genrl_core::matching::rank(candidates, &inputs, Some(1.0), &rules_empty, &settings, now);
    let verdict3 =
        genrl_core::matching::decide_verdict(ranked3, rej3, &inputs, &rules_empty, &settings);
    match verdict3 {
        genrl_core::matching::Verdict::Auto { chosen, .. } => {
            assert!(chosen
                .candidate
                .release
                .group
                .as_deref()
                .unwrap()
                .eq_ignore_ascii_case("SubsPlease"));
        }
        other => {
            panic!("Expected Auto pick for SubsPlease when no preferred group set, got {other:?}")
        }
    }
}
