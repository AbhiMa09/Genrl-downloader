use std::sync::Arc;

use genrl_core::db::Db;
use genrl_core::indexer::fetch::FakeFetch;
use genrl_core::indexer::indexer_trait::Indexer;
use genrl_core::indexer::torznab_client::TorznabClient;
use genrl_core::indexer::types::{DownloadSource, IndexerError, MediaKind, SearchQuery};

#[tokio::test]
async fn test_torznab_client_search_movie_with_caps_full() {
    let fetch = Arc::new(FakeFetch::new());
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    let movie_xml = include_str!("fixtures/torznab/search_movie.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=movie", movie_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_movie",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch.clone(),
        db,
    );

    let mut query = SearchQuery::new("Dune Part Two", MediaKind::Movie);
    query.imdb_id = Some("tt15239678".to_string());
    query.tmdb_id = Some(693134);

    let results = client.search(&query).await.expect("search must succeed");
    assert_eq!(results.len(), 3);

    // Item 1: magneturl preferred and leechers calculation (50 peers - 45 seeders = 5)
    assert_eq!(
        results[0].title,
        "Dune.Part.Two.2024.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX"
    );
    assert!(matches!(results[0].download, DownloadSource::Magnet(_)));
    assert_eq!(results[0].seeders, 45);
    assert_eq!(results[0].leechers, 5);
    assert_eq!(results[0].categories, vec![2000, 2040]);

    // Item 2: enclosure url only
    assert_eq!(
        results[1].title,
        "Oppenheimer.2023.2160p.UHD.BluRay.x265-TERMINAL"
    );
    assert!(matches!(results[1].download, DownloadSource::TorrentUrl(_)));
    assert_eq!(results[1].seeders, 20);
    assert_eq!(results[1].leechers, 5);

    // Item 3: missing seeders attribute defaults to 0
    assert_eq!(results[2].title, "Poor.Things.2023.1080p.WEBRip.x264-YTS");
    assert_eq!(results[2].seeders, 0);
    assert_eq!(results[2].leechers, 0);
}

#[tokio::test]
async fn test_torznab_client_search_movie_with_caps_minimal_mode_unavailable() {
    let fetch = Arc::new(FakeFetch::new());
    let caps_xml = include_str!("fixtures/torznab/caps_minimal.xml");

    fetch.set_response("t=caps", caps_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_minimal",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch,
        db,
    );

    let query = SearchQuery::new("Dune Part Two", MediaKind::Movie);
    let results = client.search(&query).await.expect("search must succeed");
    assert!(
        results.is_empty(),
        "when movie-search is unavailable in caps, search must return empty"
    );
}

#[tokio::test]
async fn test_torznab_client_search_tv_with_caps_full() {
    let fetch = Arc::new(FakeFetch::new());
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    let tv_xml = include_str!("fixtures/torznab/search_tv.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=tvsearch", tv_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_tv",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch.clone(),
        db,
    );

    let mut query = SearchQuery::new("Shogun", MediaKind::Series);
    query.season = Some(1);
    query.episode = Some(1.0);
    query.imdb_id = Some("tt2788310".to_string());

    let results = client.search(&query).await.expect("tv search must succeed");
    assert_eq!(results.len(), 3);

    // Verifies batch/season pack is parsed
    assert_eq!(
        results[1].title,
        "Shogun.2024.Season.1.Complete.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX"
    );
    assert_eq!(results[1].seeders, 120);
    assert_eq!(results[1].leechers, 20); // 140 peers - 120 seeders = 20 leechers
}

#[tokio::test]
async fn test_torznab_client_error_xml_maps_to_remote_error() {
    let fetch = Arc::new(FakeFetch::new());
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    let error_xml = include_str!("fixtures/torznab/error.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=movie", error_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_err",
        "http://127.0.0.1:9696",
        "INVALID_KEY",
        fetch,
        db,
    );

    let query = SearchQuery::new("Dune Part Two", MediaKind::Movie);
    let err = client
        .search(&query)
        .await
        .expect_err("must return Remote error");
    match err {
        IndexerError::Remote { code, message } => {
            assert_eq!(code, 100);
            assert_eq!(message, "Invalid API Key");
        }
        other => panic!("expected Remote error, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_torznab_client_search_anime_reads_caps_5000_range_categories() {
    let fetch = Arc::new(FakeFetch::new());
    let caps_xml = include_str!("fixtures/torznab/caps_full.xml");
    let tv_xml = include_str!("fixtures/torznab/search_tv.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=tvsearch", tv_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_anime",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch,
        db,
    );

    let query = SearchQuery::new("Frieren", MediaKind::Anime);
    let results = client
        .search(&query)
        .await
        .expect("anime search must succeed");
    assert_eq!(results.len(), 3);
}

#[tokio::test]
async fn test_torznab_client_search_omits_cat_when_no_categories_in_range() {
    let fetch = Arc::new(FakeFetch::new());
    // Caps with only category 1000 and 3000 (none in 2000-2999 or 5000-5999)
    let caps_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <caps>
        <searching>
            <movie-search available="yes" supportedParams="q,imdbid"/>
            <tv-search available="yes" supportedParams="q,season,ep"/>
        </searching>
        <categories>
            <category id="1000" name="Console"/>
            <category id="3000" name="Audio"/>
        </categories>
    </caps>"#;
    let movie_xml = include_str!("fixtures/torznab/search_movie.xml");
    let tv_xml = include_str!("fixtures/torznab/search_tv.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=movie", movie_xml);
    fetch.set_response("t=tvsearch", tv_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_no_cat",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch.clone(),
        db,
    );

    let movie_query = SearchQuery::new("Inception", MediaKind::Movie);
    let _ = client
        .search(&movie_query)
        .await
        .expect("search must succeed");

    let tv_query = SearchQuery::new("Breaking Bad", MediaKind::Series);
    let _ = client.search(&tv_query).await.expect("search must succeed");

    let requests = fetch.recorded_requests();
    let movie_req = requests.iter().find(|r| r.contains("t=movie")).unwrap();
    assert!(
        !movie_req.contains("cat="),
        "movie request must omit cat param: {movie_req}"
    );

    let tv_req = requests.iter().find(|r| r.contains("t=tvsearch")).unwrap();
    assert!(
        !tv_req.contains("cat="),
        "tv request must omit cat param: {tv_req}"
    );
}

#[tokio::test]
async fn test_torznab_client_search_uses_q_when_no_id_params_supported() {
    let fetch = Arc::new(FakeFetch::new());
    // Caps with only q supported for both movie and tv
    let caps_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
    <caps>
        <searching>
            <movie-search available="yes" supportedParams="q"/>
            <tv-search available="yes" supportedParams="q"/>
        </searching>
        <categories>
            <category id="2000" name="Movies"/>
            <category id="5000" name="TV"/>
        </categories>
    </caps>"#;
    let movie_xml = include_str!("fixtures/torznab/search_movie.xml");
    let tv_xml = include_str!("fixtures/torznab/search_tv.xml");

    fetch.set_response("t=caps", caps_xml);
    fetch.set_response("t=movie", movie_xml);
    fetch.set_response("t=tvsearch", tv_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "prowlarr_q_only",
        "http://127.0.0.1:9696",
        "DUMMY_KEY",
        fetch.clone(),
        db,
    );

    // With IDs populated in query:
    let mut movie_query = SearchQuery::new("Inception", MediaKind::Movie);
    movie_query.imdb_id = Some("tt1375666".to_string());
    movie_query.tmdb_id = Some(27205);
    let _ = client
        .search(&movie_query)
        .await
        .expect("movie search must succeed");

    let mut tv_query = SearchQuery::new("Breaking Bad", MediaKind::Series);
    tv_query.imdb_id = Some("tt0903747".to_string());
    tv_query.tvdb_id = Some(81189);
    tv_query.tmdb_id = Some(1396);
    let _ = client
        .search(&tv_query)
        .await
        .expect("tv search must succeed");

    // Without IDs in query:
    let movie_no_id = SearchQuery::new("Interstellar", MediaKind::Movie);
    let _ = client
        .search(&movie_no_id)
        .await
        .expect("movie search must succeed");

    let tv_no_id = SearchQuery::new("The Wire", MediaKind::Series);
    let _ = client
        .search(&tv_no_id)
        .await
        .expect("tv search must succeed");

    let requests = fetch.recorded_requests();
    let movie_reqs: Vec<&String> = requests.iter().filter(|r| r.contains("t=movie")).collect();
    for r in movie_reqs {
        assert!(r.contains("q="), "movie req must use q: {r}");
        assert!(
            !r.contains("imdbid="),
            "movie req must not contain imdbid: {r}"
        );
        assert!(
            !r.contains("tmdbid="),
            "movie req must not contain tmdbid: {r}"
        );
    }

    let tv_reqs: Vec<&String> = requests
        .iter()
        .filter(|r| r.contains("t=tvsearch"))
        .collect();
    for r in tv_reqs {
        assert!(r.contains("q="), "tv req must use q: {r}");
        assert!(
            !r.contains("imdbid="),
            "tv req must not contain imdbid: {r}"
        );
        assert!(
            !r.contains("tvdbid="),
            "tv req must not contain tvdbid: {r}"
        );
        assert!(
            !r.contains("tmdbid="),
            "tv req must not contain tmdbid: {r}"
        );
    }
}

#[test]
fn test_indexer_edit_and_remove_invalidates_caps_cache() {
    let db = Db::open_in_memory().unwrap();
    let now = 100000;
    let raw_xml = include_str!("fixtures/torznab/caps_full.xml");

    // Save caps to SQLite cache
    genrl_core::indexer::torznab_caps::save_cached_caps(&db, "test_idx", raw_xml, now)
        .expect("save cached caps");
    let cached = genrl_core::indexer::torznab_caps::get_cached_caps(&db, "test_idx", now)
        .expect("read cached caps");
    assert!(cached.is_some());

    // Edit indexer config -> invalidates cache
    let cfg = genrl_core::indexer::config::IndexerConfig::new(
        "test_idx",
        "Test Name",
        "http://localhost:9696",
        None,
        vec![MediaKind::Movie],
    );
    genrl_core::indexer::config::IndexerConfig::save(&db, &cfg).expect("save config");

    let cached_after_edit =
        genrl_core::indexer::torznab_caps::get_cached_caps(&db, "test_idx", now)
            .expect("read cached caps");
    assert!(
        cached_after_edit.is_none(),
        "edit must invalidate caps cache"
    );

    // Re-cache and remove -> invalidates cache
    genrl_core::indexer::torznab_caps::save_cached_caps(&db, "test_idx", raw_xml, now)
        .expect("save cached caps");
    assert!(
        genrl_core::indexer::torznab_caps::get_cached_caps(&db, "test_idx", now)
            .unwrap()
            .is_some()
    );

    genrl_core::indexer::config::IndexerConfig::remove(&db, "test_idx").expect("remove config");
    let cached_after_remove =
        genrl_core::indexer::torznab_caps::get_cached_caps(&db, "test_idx", now)
            .expect("read cached caps");
    assert!(
        cached_after_remove.is_none(),
        "remove must invalidate caps cache"
    );
}

#[tokio::test]
async fn test_error_response_not_cached_in_db() {
    let fetch = Arc::new(FakeFetch::new());
    let error_xml = r#"<?xml version="1.0" encoding="UTF-8"?><error code="100" description="Invalid API Key"/>"#;
    fetch.set_response("t=caps", error_xml);

    let db = Db::open_in_memory().unwrap();
    let client = TorznabClient::new(
        "bad_idx",
        "http://127.0.0.1:9696",
        "BAD_KEY",
        fetch,
        db.clone(),
    );

    let query = SearchQuery::new("Inception", MediaKind::Movie);
    let err = client.search(&query).await.expect_err("search should fail");
    assert!(matches!(err, IndexerError::Remote { code: 100, .. }));

    // Verify DB does not contain any cached caps for bad_idx
    let cached = genrl_core::indexer::torznab_caps::get_cached_caps(&db, "bad_idx", 100000)
        .expect("get cached caps");
    assert!(
        cached.is_none(),
        "error response must never be cached in SQLite"
    );
}

#[test]
fn test_indexer_config_key_never_written_to_db() {
    let db = Db::open_in_memory().unwrap();
    let secret = "VERY_SECRET_KEY_NEVER_IN_DB_123456789";
    let cfg = genrl_core::indexer::config::IndexerConfig::new(
        "prowlarr_sec",
        "Prowlarr",
        "http://127.0.0.1:9696",
        Some(secret.to_string()),
        vec![MediaKind::Movie],
    );
    genrl_core::indexer::config::IndexerConfig::save(&db, &cfg).expect("save must succeed");

    // Scan all tables and columns in SQLite for the secret string
    db.with(|c| {
        let mut stmt = c.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(std::result::Result::ok)
            .collect();
        for table in tables {
            let mut row_stmt = c.prepare(&format!("SELECT * FROM \"{table}\""))?;
            let col_count = row_stmt.column_count();
            let mut rows = row_stmt.query([])?;
            while let Some(row) = rows.next()? {
                for i in 0..col_count {
                    if let Ok(val) = row.get::<_, String>(i) {
                        assert!(
                            !val.contains(secret),
                            "secret found in table {table} column {i}"
                        );
                    }
                }
            }
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn test_indexer_config_secrets_file_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    let secrets_path = temp_dir.path().join("secrets.json");
    std::env::set_var("GENRL_FORCE_NO_KEYRING", "1");
    std::env::set_var("GENRL_SECRETS_FILE", secrets_path.to_str().unwrap());

    genrl_core::indexer::config::save_api_key("test_idx", "my_file_secret");
    let loaded = genrl_core::indexer::config::load_api_key("test_idx");
    assert_eq!(loaded.as_deref(), Some("my_file_secret"));

    // File exists and contains the secret
    let content = std::fs::read_to_string(&secrets_path).unwrap();
    assert!(content.contains("my_file_secret"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::metadata(&secrets_path).unwrap().permissions();
        assert_eq!(
            perms.mode() & 0o777,
            0o600,
            "secrets file must have mode 0600"
        );
    }

    genrl_core::indexer::config::delete_api_key("test_idx");
    let after_delete = genrl_core::indexer::config::load_api_key("test_idx");
    assert_eq!(after_delete, None);

    std::env::remove_var("GENRL_SECRETS_FILE");
    std::env::remove_var("GENRL_FORCE_NO_KEYRING");
}

#[test]
fn test_candidate_and_release_debug_and_serialize_redact_secrets() {
    let download = DownloadSource::TorrentUrl(
        "https://prowlarr.local/dl?apikey=SECRET_KEY_123&passkey=PASS_456&torrent_pass=PASS_789"
            .to_string(),
    );
    let release = genrl_core::indexer::types::Release {
        title: "Test Release".to_string(),
        indexer_id: "idx_1".to_string(),
        size_bytes: 1024,
        seeders: 10,
        leechers: 2,
        download,
        info_hash: Some("0123456789abcdef".to_string()),
        categories: vec![2000],
        native_category: None,
        published_date: None,
        trusted: true,
        remake: false,
    };

    let rel_debug = format!("{release:?}");
    assert!(
        !rel_debug.contains("SECRET_KEY_123"),
        "release debug leaked apikey"
    );
    assert!(
        !rel_debug.contains("PASS_456"),
        "release debug leaked passkey"
    );
    assert!(
        !rel_debug.contains("PASS_789"),
        "release debug leaked torrent_pass"
    );
    assert!(rel_debug.contains("[REDACTED]"));

    let rel_json = serde_json::to_string(&release).unwrap();
    assert!(
        !rel_json.contains("SECRET_KEY_123"),
        "release json leaked apikey"
    );
    assert!(
        !rel_json.contains("PASS_456"),
        "release json leaked passkey"
    );
    assert!(
        !rel_json.contains("PASS_789"),
        "release json leaked torrent_pass"
    );
    assert!(rel_json.contains("[REDACTED]"));

    let parsed_rel = genrl_core::filename::parse_release("Test Release 1080p");
    let candidate = genrl_core::score::types::Candidate {
        release: parsed_rel,
        info_hash: "0123456789abcdef".to_string(),
        magnet:
            "magnet:?xt=urn:btih:0123456789abcdef&apikey=SECRET_CANDIDATE&passkey=PASS_CANDIDATE"
                .to_string(),
        seeders: 10,
        leechers: 2,
        trusted: true,
        remake: false,
        size_bytes: 1024,
        published_at: 10000,
        uploader: None,
        show_type: None,
        category_id: None,
    };

    let cand_debug = format!("{candidate:?}");
    assert!(
        !cand_debug.contains("SECRET_CANDIDATE"),
        "candidate debug leaked apikey"
    );
    assert!(
        !cand_debug.contains("PASS_CANDIDATE"),
        "candidate debug leaked passkey"
    );
    assert!(cand_debug.contains("[REDACTED]"));
}
