use std::sync::Arc;

use genrl_core::db::media_repo::{find_by_id, find_by_tmdb_id, upsert, UpsertMedia};
use genrl_core::db::Db;
use genrl_core::indexer::fetch::FakeFetch;
use genrl_core::indexer::types::MediaKind;
use genrl_core::metadata::provider_trait::MetadataProvider;
use genrl_core::metadata::tmdb::TmdbClient;

#[tokio::test]
async fn test_tmdb_search_movie() {
    let fetch = Arc::new(FakeFetch::new());
    let movie_json = include_str!("fixtures/tmdb/search_movie.json");
    fetch.set_response("/search/movie", movie_json);

    let client = TmdbClient::new("TESTKEY", fetch);
    let results = client
        .search("Dune: Part Two", MediaKind::Movie, Some(2024))
        .await
        .expect("search must succeed");

    assert_eq!(results.len(), 1);
    let movie = &results[0];
    assert_eq!(movie.kind, MediaKind::Movie);
    assert_eq!(movie.tmdb_id, 693134);
    assert_eq!(movie.title, "Dune: Part Two");
    assert_eq!(movie.original_title.as_deref(), Some("Dune: Part Two"));
    assert_eq!(movie.year, Some(2024));
    assert_eq!(
        movie.poster_path.as_deref(),
        Some("/1pdfLvkbY9ohJlCjQH2CZjjYVvJ.jpg")
    );
}

#[tokio::test]
async fn test_tmdb_search_tv() {
    let fetch = Arc::new(FakeFetch::new());
    let tv_json = include_str!("fixtures/tmdb/search_tv.json");
    fetch.set_response("/search/tv", tv_json);

    let client = TmdbClient::new("TESTKEY", fetch);
    let results = client
        .search("Shōgun", MediaKind::Series, Some(2024))
        .await
        .expect("tv search must succeed");

    assert_eq!(results.len(), 1);
    let tv = &results[0];
    assert_eq!(tv.kind, MediaKind::Series);
    assert_eq!(tv.tmdb_id, 126308);
    assert_eq!(tv.title, "Shōgun");
    assert_eq!(tv.original_title.as_deref(), Some("Shōgun"));
    assert_eq!(tv.year, Some(2024));
}

#[tokio::test]
async fn test_tmdb_get_external_ids_movie() {
    let fetch = Arc::new(FakeFetch::new());
    let ext_json = include_str!("fixtures/tmdb/external_ids_movie.json");
    fetch.set_response("/movie/693134/external_ids", ext_json);

    let client = TmdbClient::new("TESTKEY", fetch);
    let ids = client
        .get_external_ids(693134, MediaKind::Movie)
        .await
        .expect("must get movie external IDs");

    assert_eq!(ids.tmdb_id, Some(693134));
    assert_eq!(ids.imdb_id.as_deref(), Some("tt15239678"));
    assert_eq!(ids.tvdb_id, None);
}

#[tokio::test]
async fn test_tmdb_get_external_ids_tv() {
    let fetch = Arc::new(FakeFetch::new());
    let ext_json = include_str!("fixtures/tmdb/external_ids_tv.json");
    fetch.set_response("/tv/126308/external_ids", ext_json);

    let client = TmdbClient::new("TESTKEY", fetch);
    let ids = client
        .get_external_ids(126308, MediaKind::Series)
        .await
        .expect("must get tv external IDs");

    assert_eq!(ids.tmdb_id, Some(126308));
    assert_eq!(ids.imdb_id.as_deref(), Some("tt2788310"));
    assert_eq!(ids.tvdb_id, Some(361730));
}

#[tokio::test]
async fn test_tmdb_get_season_episodes() {
    let fetch = Arc::new(FakeFetch::new());
    let season_json = include_str!("fixtures/tmdb/tv_season.json");
    fetch.set_response("/tv/126308/season/1", season_json);

    let client = TmdbClient::new("TESTKEY", fetch);
    let episodes = client
        .get_season_episodes(126308, 1)
        .await
        .expect("must get season episodes");

    assert_eq!(episodes.len(), 2);
    assert_eq!(episodes[0].season, 1);
    assert_eq!(episodes[0].episode, 1);
    assert_eq!(episodes[0].title, "Chapter One: Anjin");
    assert_eq!(episodes[0].air_date.as_deref(), Some("2024-02-27"));

    assert_eq!(episodes[1].season, 1);
    assert_eq!(episodes[1].episode, 2);
    assert_eq!(episodes[1].title, "Chapter Two: Servants of Two Masters");
}

#[tokio::test]
async fn test_tmdb_error_redacts_api_key() {
    let fetch = Arc::new(FakeFetch::new());
    fetch.set_error(
        "/search/movie",
        "HTTP 401 Unauthorized for https://api.themoviedb.org/3/search/movie?api_key=SECRET_TMDB_KEY_999",
    );

    let client = TmdbClient::new("SECRET_TMDB_KEY_999", fetch);
    let err = client
        .search("Inception", MediaKind::Movie, None)
        .await
        .expect_err("must error");

    let err_str = err.to_string();
    assert!(
        !err_str.contains("SECRET_TMDB_KEY_999"),
        "error leaked TMDB API key: {err_str}"
    );
    assert!(
        err_str.contains("[REDACTED]"),
        "error must contain [REDACTED]: {err_str}"
    );
}

#[test]
fn test_media_repo_upsert_and_find() {
    let db = Db::open_in_memory().unwrap();
    let now = 1710000000;

    let mut item = UpsertMedia::new(MediaKind::Series, "Shōgun");
    item.tmdb_id = Some(126308);
    item.imdb_id = Some("tt2788310".to_string());
    item.tvdb_id = Some(361730);
    item.year = Some(2024);
    item.total_seasons = Some(1);
    item.total_episodes = Some(10);
    item.status = Some("Ended".to_string());

    let id = upsert(&db, &item, now).expect("upsert series");
    let record = find_by_id(&db, id).expect("find").expect("exists");
    assert_eq!(record.title, "Shōgun");
    assert_eq!(record.kind, MediaKind::Series);
    assert_eq!(record.tmdb_id, Some(126308));
    assert_eq!(record.imdb_id.as_deref(), Some("tt2788310"));
    assert_eq!(record.tvdb_id, Some(361730));
    assert_eq!(record.total_seasons, Some(1));
    assert_eq!(record.total_episodes, Some(10));
    assert_eq!(record.status.as_deref(), Some("Ended"));

    let by_tmdb = find_by_tmdb_id(&db, MediaKind::Series, 126308)
        .expect("find by tmdb")
        .expect("exists");
    assert_eq!(by_tmdb.id, id);

    // Idempotent upsert: updating poster and status doesn't duplicate
    item.poster_path = Some("/new_poster.jpg".to_string());
    item.status = Some("Returning Series".to_string());
    let id_updated = upsert(&db, &item, now + 500).expect("second upsert");
    assert_eq!(id, id_updated);

    let updated = find_by_id(&db, id).expect("find").expect("exists");
    assert_eq!(updated.poster_path.as_deref(), Some("/new_poster.jpg"));
    assert_eq!(updated.status.as_deref(), Some("Returning Series"));
    assert_eq!(updated.updated_at, Some(now + 500));
}
