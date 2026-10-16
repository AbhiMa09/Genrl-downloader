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
