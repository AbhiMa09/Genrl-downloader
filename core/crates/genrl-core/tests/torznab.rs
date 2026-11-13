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
