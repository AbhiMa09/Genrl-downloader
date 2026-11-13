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
    let score_cam =
        genrl_core::score::evaluate(&cam, &target, &rules, &settings, 1_700_000_000).unwrap();

    assert!(
        score_bluray.total() > score_cam.total(),
        "BluRay ({:.1}) must beat CAM ({:.1}) even if CAM has 100x more seeders",
        score_bluray.total(),
        score_cam.total()
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
