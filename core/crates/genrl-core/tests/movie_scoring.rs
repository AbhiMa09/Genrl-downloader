use genrl_core::matching::release_parse::parse_media_release;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TestCase {
    raw: String,
    title: String,
    year: Option<u32>,
    season: Option<u32>,
    episode: Option<f64>,
    season_range: Option<[u32; 2]>,
    episode_range: Option<[f64; 2]>,
    resolution: Option<String>,
    source: Option<String>,
    codec: Option<String>,
    audio: Option<String>,
    group: Option<String>,
    is_cam: bool,
    is_proper: bool,
    is_repack: bool,
    is_hardcoded_subs: bool,
    is_batch: bool,
}

#[test]
fn test_release_name_table_parsing() {
    let json_str = include_str!("fixtures/release_names/release_table.json");
    let cases: Vec<TestCase> =
        serde_json::from_str(json_str).expect("release_table.json must be valid JSON");

    assert!(
        cases.len() >= 30,
        "Table must contain at least 30 real-world test cases"
    );

    for (idx, case) in cases.iter().enumerate() {
        let parsed = parse_media_release(&case.raw);

        assert_eq!(
            parsed.title, case.title,
            "Case {}: title mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.year, case.year,
            "Case {}: year mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.season, case.season,
            "Case {}: season mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.episode, case.episode,
            "Case {}: episode mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.season_range,
            case.season_range.map(|[s1, s2]| (s1, s2)),
            "Case {}: season_range mismatch for '{}'",
            idx,
            case.raw
        );
        assert_eq!(
            parsed.episode_range,
            case.episode_range.map(|[e1, e2]| (e1, e2)),
            "Case {}: episode_range mismatch for '{}'",
            idx,
            case.raw
        );
        assert_eq!(
            parsed.resolution, case.resolution,
            "Case {}: resolution mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.source, case.source,
            "Case {}: source mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.codec, case.codec,
            "Case {}: codec mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.audio, case.audio,
            "Case {}: audio mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.group, case.group,
            "Case {}: group mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.is_cam, case.is_cam,
            "Case {}: is_cam mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.is_proper, case.is_proper,
            "Case {}: is_proper mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.is_repack, case.is_repack,
            "Case {}: is_repack mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.is_hardcoded_subs, case.is_hardcoded_subs,
            "Case {}: is_hardcoded_subs mismatch for '{}'",
            idx, case.raw
        );
        assert_eq!(
            parsed.is_batch, case.is_batch,
            "Case {}: is_batch mismatch for '{}'",
            idx, case.raw
        );
    }
}

fn mock_cand(raw: &str, seeders: u32) -> genrl_core::score::types::Candidate {
    genrl_core::score::types::Candidate {
        release: genrl_core::filename::parse_release(raw),
        info_hash: "1111111111111111111111111111111111111111".to_string(),
        magnet: "magnet:?xt=urn:btih:1111111111111111111111111111111111111111".to_string(),
        seeders,
        leechers: 10,
        trusted: true,
        remake: false,
        size_bytes: 4_000_000_000,
        published_at: 1_700_000_000,
        uploader: None,
        show_type: None,
        category_id: None,
    }
}

#[test]
fn test_bluray_beats_cam_with_more_seeders() {
    let names = vec!["Dune Part Two".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    let bluray = mock_cand("Dune.Part.Two.2024.1080p.BluRay.x264-SPARKS", 50);
    let cam = mock_cand("Dune.Part.Two.2024.CAMRip.x264-CAM", 5000);

    let rules = genrl_core::rules::Rules::default();
    let settings = genrl_core::settings::Settings::default();

    let score_bluray =
        genrl_core::score::evaluate(&bluray, &target, &rules, &settings, 1_700_000_000).unwrap();
    let cam_res = genrl_core::score::evaluate(&cam, &target, &rules, &settings, 1_700_000_000);

    assert!(score_bluray.total() > 0.0);
    assert_eq!(
        cam_res,
        Err(genrl_core::score::types::Rejection::LowQualitySource {
            source: "camrip".to_string()
        })
    );
}

#[test]
fn test_wrong_season_rejected() {
    let names = vec!["Shogun".to_string()];
    let others = vec![];
    let target_s2 = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: genrl_core::models::ShowType::Tv,
        season: 2,
        start_year: Some(2024),
        fully_aired: false,
        total_episodes: Some(10),
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Series),
    };

    let cand_s1 = mock_cand("Shogun.2024.S01E01.1080p.WEB-DL.H.264-FLUX", 100);
    let err = genrl_core::score::evaluate(
        &cand_s1,
        &target_s2,
        &genrl_core::rules::Rules::default(),
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap_err();
    assert_eq!(
        err,
        genrl_core::score::types::Rejection::SeasonConflict {
            wanted: 2,
            found: 1
        }
    );
}

#[test]
fn test_hardsub_penalized_when_softsub_preferred() {
    let names = vec!["Gladiator II".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    let clean = mock_cand("Gladiator.II.2024.1080p.HDRip.x264-FLUX", 50);
    let hardsub = mock_cand("Gladiator.II.2024.1080p.KORSUB.HDRip.x264-STUTTERSHIT", 50);

    let score_clean = genrl_core::score::evaluate(
        &clean,
        &target,
        &genrl_core::rules::Rules::default(),
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();
    let score_hardsub = genrl_core::score::evaluate(
        &hardsub,
        &target,
        &genrl_core::rules::Rules::default(),
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();

    assert!(
        score_clean.total() > score_hardsub.total(),
        "Clean release ({:.1}) must outscore hardsub release ({:.1})",
        score_clean.total(),
        score_hardsub.total()
    );
}

#[test]
fn test_exact_year_bonus() {
    let names = vec!["Civil War".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    let with_year = mock_cand("Civil.War.2024.1080p.WEB-DL.H.264-FLUX", 50);
    let no_year = mock_cand("Civil.War.1080p.WEB-DL.H.264-FLUX", 50);

    let score_with_year = genrl_core::score::evaluate(
        &with_year,
        &target,
        &genrl_core::rules::Rules::default(),
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();
    let score_no_year = genrl_core::score::evaluate(
        &no_year,
        &target,
        &genrl_core::rules::Rules::default(),
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();

    assert!(
        score_with_year.signal > score_no_year.signal,
        "Release with exact matching year must receive year bonus: {:.1} vs {:.1}",
        score_with_year.signal,
        score_no_year.signal
    );
}

#[test]
fn test_low_quality_sources_rejected() {
    let spellings = [
        ("Movie.2024.CAM.x264", "cam"),
        ("Movie.2024.HDCAM.x264", "hdcam"),
        ("Movie.2024.TS.x264", "ts"),
        ("Movie.2024.HDTS.x264", "hdts"),
        ("Movie.2024.HD-TS.x264", "hdts"),
        ("Movie.2024.TELESYNC.x264", "telesync"),
        ("Movie.2024.TC.x264", "tc"),
        ("Movie.2024.TELECINE.x264", "telecine"),
        ("Movie.2024.SCR.x264", "scr"),
        ("Movie.2024.DVDSCR.x264", "scr"),
    ];

    let names = vec!["Movie".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    for (raw, expected_src) in spellings {
        let parsed = parse_media_release(raw);
        assert!(parsed.is_cam, "parser must flag {} as is_cam", raw);
        assert_eq!(
            parsed.source.as_deref(),
            Some(expected_src),
            "source mismatch for {}",
            raw
        );

        let cand = mock_cand(raw, 500);
        let res = genrl_core::score::evaluate(
            &cand,
            &target,
            &genrl_core::rules::Rules::default(),
            &genrl_core::settings::Settings::default(),
            1_700_000_000,
        );
        assert_eq!(
            res,
            Err(genrl_core::score::types::Rejection::LowQualitySource {
                source: expected_src.to_string()
            }),
            "evaluation must reject {} with LowQualitySource",
            raw
        );
    }
}

#[test]
fn test_movie_size_sanity_checks() {
    let names = vec!["Dune Part Two".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    let rules = genrl_core::rules::Rules::default(); // movie_min: 300 MB, movie_max: 80 GB
    let settings = genrl_core::settings::Settings::default();

    // 1. Movie below 300 MB (250 MB) -> Rejected
    let mut small = mock_cand("Dune.Part.Two.2024.1080p.WEB-DL.H.264-FLUX", 50);
    small.size_bytes = 250 * 1024 * 1024;
    assert_eq!(
        genrl_core::score::evaluate(&small, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::SizeOutOfRange {
            size_mb: 250,
            min_mb: 300,
            max_mb: 81_920,
        })
    );

    // 2. Movie above 80 GB (85 GB) -> Rejected
    let mut large = mock_cand("Dune.Part.Two.2024.1080p.WEB-DL.H.264-FLUX", 50);
    large.size_bytes = 85 * 1024 * 1024 * 1024;
    assert_eq!(
        genrl_core::score::evaluate(&large, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::SizeOutOfRange {
            size_mb: 85 * 1024,
            min_mb: 300,
            max_mb: 81_920,
        })
    );

    // 3. Movie valid (4 GB) -> Accepted
    let mut ok_movie = mock_cand("Dune.Part.Two.2024.1080p.WEB-DL.H.264-FLUX", 50);
    ok_movie.size_bytes = 4 * 1024 * 1024 * 1024;
    assert!(
        genrl_core::score::evaluate(&ok_movie, &target, &rules, &settings, 1_700_000_000).is_ok()
    );
}

#[test]
fn test_series_size_sanity_checks() {
    let names = vec!["Shogun".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(1.0),
        show_type: genrl_core::models::ShowType::Tv,
        season: 1,
        start_year: Some(2024),
        fully_aired: false,
        total_episodes: Some(10),
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Series),
    };

    let rules = genrl_core::rules::Rules::default(); // episode_min: 80 MB, episode_max: 15 GB
    let settings = genrl_core::settings::Settings::default();

    // 1. Episode below 80 MB (50 MB) -> Rejected
    let mut small_ep = mock_cand("Shogun.2024.S01E01.1080p.WEB-DL.H.264-FLUX", 50);
    small_ep.size_bytes = 50 * 1024 * 1024;
    assert_eq!(
        genrl_core::score::evaluate(&small_ep, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::SizeOutOfRange {
            size_mb: 50,
            min_mb: 80,
            max_mb: 15_360,
        })
    );

    // 2. Episode above 15 GB (18 GB) -> Rejected
    let mut huge_ep = mock_cand("Shogun.2024.S01E01.1080p.WEB-DL.H.264-FLUX", 50);
    huge_ep.size_bytes = 18 * 1024 * 1024 * 1024;
    assert_eq!(
        genrl_core::score::evaluate(&huge_ep, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::SizeOutOfRange {
            size_mb: 18 * 1024,
            min_mb: 80,
            max_mb: 15_360,
        })
    );

    // 3. Episode valid (1.5 GB) -> Accepted
    let mut ok_ep = mock_cand("Shogun.2024.S01E01.1080p.WEB-DL.H.264-FLUX", 50);
    ok_ep.size_bytes = 1500 * 1024 * 1024;
    assert!(genrl_core::score::evaluate(&ok_ep, &target, &rules, &settings, 1_700_000_000).is_ok());

    // 4. Batch scaled by episode count (10 eps -> 800 MB to 150 GB)
    let mut batch = mock_cand("Shogun.2024.Season.1.Complete.1080p.WEB-DL.H.264-FLUX", 50);
    batch.size_bytes = 15 * 1024 * 1024 * 1024; // 15 GB for 10 eps
    assert!(genrl_core::score::evaluate(&batch, &target, &rules, &settings, 1_700_000_000).is_ok());

    let mut too_small_batch =
        mock_cand("Shogun.2024.Season.1.Complete.1080p.WEB-DL.H.264-FLUX", 50);
    too_small_batch.size_bytes = 500 * 1024 * 1024; // 500 MB for 10 eps (< 800 MB min)
    assert!(matches!(
        genrl_core::score::evaluate(&too_small_batch, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::SizeOutOfRange { .. })
    ));
}

#[test]
fn test_series_episode_mismatch_and_batch_coverage() {
    let names = vec!["Fallout".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: Some(3.0),
        show_type: genrl_core::models::ShowType::Tv,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: Some(8),
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Series),
    };

    let rules = genrl_core::rules::Rules::default();
    let settings = genrl_core::settings::Settings::default();

    // 1. Single episode matching (E03) -> Accepted
    let ep3 = mock_cand("Fallout.S01E03.1080p.WEB-DL.H.264-FLUX", 50);
    assert!(genrl_core::score::evaluate(&ep3, &target, &rules, &settings, 1_700_000_000).is_ok());

    // 2. Single episode mismatch (E05 when wanted 3) -> Rejected
    let ep5 = mock_cand("Fallout.S01E05.1080p.WEB-DL.H.264-FLUX", 50);
    assert_eq!(
        genrl_core::score::evaluate(&ep5, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::WrongEpisode {
            wanted: 3.0,
            found: 5.0,
        })
    );

    // 3. Batch covering wanted episode (E01-E04 contains 3) -> Accepted
    let batch_ok = mock_cand("Fallout.S01E01-E04.1080p.WEB-DL.H.264-FLUX", 50);
    assert!(
        genrl_core::score::evaluate(&batch_ok, &target, &rules, &settings, 1_700_000_000).is_ok()
    );

    // 4. Batch not covering wanted episode (E05-E08 does not contain 3) -> Rejected
    let batch_miss = mock_cand("Fallout.S01E05-E08.1080p.WEB-DL.H.264-FLUX", 50);
    assert_eq!(
        genrl_core::score::evaluate(&batch_miss, &target, &rules, &settings, 1_700_000_000),
        Err(genrl_core::score::types::Rejection::WrongEpisode {
            wanted: 3.0,
            found: 8.0,
        })
    );
}

#[test]
fn test_hardsub_penalty_toggled_by_prefer_softsub_rule() {
    let names = vec!["Gladiator II".to_string()];
    let others = vec![];
    let target = genrl_core::score::types::Target {
        names: &names,
        other_names: &others,
        wanted_episode: None,
        show_type: genrl_core::models::ShowType::Movie,
        season: 1,
        start_year: Some(2024),
        fully_aired: true,
        total_episodes: None,
        season_maps: &[],
        missing_episodes: &[],
        owned_episodes: &[],
        media_kind: Some(genrl_core::indexer::types::MediaKind::Movie),
    };

    let clean = mock_cand("Gladiator.II.2024.1080p.HDRip.x264-FLUX", 50);
    let hardsub = mock_cand("Gladiator.II.2024.1080p.KORSUB.HDRip.x264-STUTTERSHIT", 50);

    let mut rules_no_softsub_pref = genrl_core::rules::Rules::default();
    rules_no_softsub_pref.prefer_softsub = false;

    let score_clean = genrl_core::score::evaluate(
        &clean,
        &target,
        &rules_no_softsub_pref,
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();
    let score_hardsub = genrl_core::score::evaluate(
        &hardsub,
        &target,
        &rules_no_softsub_pref,
        &genrl_core::settings::Settings::default(),
        1_700_000_000,
    )
    .unwrap();

    assert_eq!(
        score_clean.ranking, score_hardsub.ranking,
        "When prefer_softsub is false, hardsub release must NOT receive ranking penalty"
    );
}

#[test]
fn test_hardcoded_sub_detection_negative_tests() {
    let neg1 = parse_media_release("Movie.2024.1080p.WEB-DL.English.Subs-GROUP");
    assert!(
        !neg1.is_hardcoded_subs,
        "Words 'Subs' must not flag hardcoded subs"
    );

    let neg2 = parse_media_release("Show.S01E01.1080p.Multi-Sub.x264-GROUP");
    assert!(
        !neg2.is_hardcoded_subs,
        "Words 'Multi-Sub' must not flag hardcoded subs"
    );

    let neg3 = parse_media_release("Film.2023.720p.Subbed.x264-GROUP");
    assert!(
        !neg3.is_hardcoded_subs,
        "Words 'Subbed' must not flag hardcoded subs"
    );
}

#[test]
fn test_parser_garbage_strings_never_panic() {
    let garbage = [
        "",
        "   ",
        "........",
        "---",
        "a",
        "1234567890",
        "🎉🚀🎬🍿🔥 Unicode emojis",
        "日本語のタイトル.2024.1080p.mkv",
        "العربية.فيلم.2024.WEB-DL.mkv",
        &"A".repeat(100_000),
    ];

    for s in garbage {
        let parsed = parse_media_release(s);
        let _ = format!("{:?}", parsed);
    }
}
