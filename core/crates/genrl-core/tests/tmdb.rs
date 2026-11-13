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

#[tokio::test]
async fn test_every_tmdb_request_sends_include_adult_false_and_language_en_us() {
    let fetch = Arc::new(FakeFetch::new());
    fetch.set_response(
        "/search/movie",
        include_str!("fixtures/tmdb/search_movie.json"),
    );
    fetch.set_response("/search/tv", include_str!("fixtures/tmdb/search_tv.json"));
    fetch.set_response(
        "/movie/693134/external_ids",
        include_str!("fixtures/tmdb/external_ids_movie.json"),
    );
    fetch.set_response(
        "/tv/126308/external_ids",
        include_str!("fixtures/tmdb/external_ids_tv.json"),
    );
    fetch.set_response(
        "/tv/126308/season/1",
        include_str!("fixtures/tmdb/tv_season.json"),
    );

    let client = TmdbClient::new("TESTKEY", fetch.clone());
    let _ = client.search("Dune", MediaKind::Movie, None).await.unwrap();
    let _ = client
        .search("Shogun", MediaKind::Series, None)
        .await
        .unwrap();
    let _ = client
        .get_external_ids(693134, MediaKind::Movie)
        .await
        .unwrap();
    let _ = client
        .get_external_ids(126308, MediaKind::Series)
        .await
        .unwrap();
    let _ = client.get_season_episodes(126308, 1).await.unwrap();

    let reqs = fetch.recorded_requests();
    assert_eq!(reqs.len(), 5);
    for req in reqs {
        assert!(
            req.contains("include_adult=false"),
            "request missing include_adult=false: {req}"
        );
        assert!(
            req.contains("language=en-US"),
            "request missing language=en-US: {req}"
        );
    }
}

#[tokio::test]
async fn test_tmdb_search_accepts_optional_year_and_sends_correct_param() {
    let fetch = Arc::new(FakeFetch::new());
    fetch.set_response(
        "/search/movie",
        include_str!("fixtures/tmdb/search_movie.json"),
    );
    fetch.set_response("/search/tv", include_str!("fixtures/tmdb/search_tv.json"));

    let client = TmdbClient::new("TESTKEY", fetch.clone());

    // 1. Movie with year -> sends year=2024
    let _ = client
        .search("Dune", MediaKind::Movie, Some(2024))
        .await
        .unwrap();
    // 2. Movie without year -> does not send year=
    let _ = client.search("Dune", MediaKind::Movie, None).await.unwrap();
    // 3. TV with year -> sends first_air_date_year=2024
    let _ = client
        .search("Shogun", MediaKind::Series, Some(2024))
        .await
        .unwrap();
    // 4. TV without year -> does not send first_air_date_year=
    let _ = client
        .search("Shogun", MediaKind::Series, None)
        .await
        .unwrap();

    let reqs = fetch.recorded_requests();
    assert!(
        reqs[0].contains("year=2024"),
        "movie with year must have year=2024"
    );
    assert!(
        !reqs[1].contains("year="),
        "movie without year must not have year="
    );
    assert!(
        reqs[2].contains("first_air_date_year=2024"),
        "tv with year must have first_air_date_year=2024"
    );
    assert!(
        !reqs[3].contains("first_air_date_year="),
        "tv without year must not have first_air_date_year="
    );
}

#[test]
fn test_season_0_specials_ignored_by_default_with_explicit_opt_in() {
    use genrl_core::metadata::tmdb_models::{TmdbSeasonSummary, TmdbTvDetailsResponse};

    let details = TmdbTvDetailsResponse {
        id: 126308,
        name: "Shōgun".to_string(),
        status: Some("Ended".to_string()),
        number_of_seasons: Some(2),
        number_of_episodes: Some(18),
        seasons: vec![
            TmdbSeasonSummary {
                id: 100,
                season_number: 0, // Specials
                name: Some("Specials".to_string()),
                episode_count: Some(4),
                air_date: Some("2024-01-01".to_string()),
            },
            TmdbSeasonSummary {
                id: 101,
                season_number: 1,
                name: Some("Season 1".to_string()),
                episode_count: Some(10),
                air_date: Some("2024-02-27".to_string()),
            },
            TmdbSeasonSummary {
                id: 102,
                season_number: 2,
                name: Some("Season 2".to_string()),
                episode_count: Some(8),
                air_date: Some("2025-01-01".to_string()),
            },
        ],
    };

    // By default (include_specials = false): season 0 is ignored
    assert_eq!(details.regular_seasons_count(false), 2);
    assert_eq!(details.total_episodes_count(false), 18);

    // With explicit opt-in (include_specials = true): season 0 is included
    assert_eq!(details.regular_seasons_count(true), 3);
    assert_eq!(details.total_episodes_count(true), 22);
}

#[test]
fn test_media_repo_upsert_same_kind_tmdb_id_updates_title_and_leaves_single_row() {
    let db = Db::open_in_memory().unwrap();
    let now = 1710000000;

    let mut item = UpsertMedia::new(MediaKind::Movie, "Dune Part 2");
    item.tmdb_id = Some(693134);

    let id1 = upsert(&db, &item, now).expect("first upsert");

    // Second upsert with updated title
    item.title = "Dune: Part Two (Official)".to_string();
    let id2 = upsert(&db, &item, now + 100).expect("second upsert");

    assert_eq!(
        id1, id2,
        "upsert with same kind and tmdb_id must update existing row"
    );

    // Ensure only 1 row exists in media table
    let count: i64 = db
        .with(|c| {
            Ok(
                c.query_row("SELECT COUNT(*) FROM media WHERE kind = 'movie'", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(count, 1, "exactly one row must remain in media table");

    let record = find_by_id(&db, id1).expect("find").expect("exists");
    assert_eq!(record.title, "Dune: Part Two (Official)");
}
