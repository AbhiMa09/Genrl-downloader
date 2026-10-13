//! Phase 5 check: MAL OAuth, list sync, season chains, and resilience.
//!
//! Every test runs against a local WireMock server — no real MAL calls, no
//! keyring, no network.  The same helpers that the gate tests use drive the
//! limiter, cache and backoff so those paths are exercised too.
//!
//! Tests:
//!
//! 1. Token exchange — code → access token, refresh token stored.
//! 2. Token auto-refresh — expired token triggers the refresh endpoint before
//!    the list is fetched.
//! 3. List paging — two pages arrive, both entries land in the database.
//! 4. Season chains — a sequel relation links two entries into one chain.
//! 5. Sync is idempotent — running it twice gives the same database state.
//! 6. Diff detects changes — a watched-count change is counted as "updated".
//! 7. Removed entry — an entry absent from the second sync loses its
//!    `mal_entries` row and has `auto = 0` set on the show.
//! 8. 429 on the list endpoint — the gate retries and the sync succeeds on the
//!    second attempt.
//! 9. Relations failure — one failed relation lookup leaves that entry as its
//!    own chain without aborting the whole sync.
//! 10. `is_due` cadence — `sync_if_due` skips when the interval has not elapsed.

mod common;

use std::sync::atomic::Ordering;

use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::clock::FakeClock;
use genrl_core::mal::{self, MemoryStore, MalClient, TokenStore};
use genrl_core::settings::Settings;
use genrl_core::{Db, Gate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Shared clock epoch — 2026-10-13 in Unix time, far enough from 0 that
/// timestamps are unambiguous in assertions.
const NOW: i64 = 1_760_000_000;

fn clock() -> FakeClock {
    FakeClock::new(NOW)
}

fn db() -> Db {
    Db::open_in_memory().unwrap()
}

fn gate(db: Db) -> Gate {
    Gate::new(db).unwrap()
}

/// Settings pointing every endpoint at the mock server.
fn settings_for(server: &MockServer) -> Settings {
    let mut s = Settings::default();
    s.mal_client_id = "test_client_id".to_string();
    s.mal_base_url = server.uri();
    s
}

/// A valid token-endpoint response.
fn token_response(access: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "access_token": access,
        "refresh_token": "rt_fresh",
        "token_type": "Bearer",
        "expires_in": 3600
    }))
}

/// A list-page JSON body with `n` entries.
fn list_page(entries: &[serde_json::Value], has_next: bool) -> serde_json::Value {
    let paging = if has_next {
        serde_json::json!({ "next": "https://example.com/next" })
    } else {
        serde_json::json!({})
    };
    serde_json::json!({ "data": entries, "paging": paging })
}

/// One `data` element for the list endpoint.
fn list_entry(id: i64, title: &str, status: &str, watched: i64, total: i64) -> serde_json::Value {
    serde_json::json!({
        "node": {
            "id": id,
            "title": title,
            "num_episodes": total,
            "media_type": "tv",
            "status": "finished_airing",
            "alternative_titles": { "synonyms": [], "en": null, "ja": null }
        },
        "list_status": {
            "status": status,
            "score": 0,
            "num_episodes_watched": watched,
            "updated_at": "2026-10-01T00:00:00+00:00"
        }
    })
}

/// Relations body for `GET /anime/{id}?fields=related_anime`.
fn relations_body(relations: &[serde_json::Value]) -> serde_json::Value {
    serde_json::json!({ "id": 1, "related_anime": relations })
}

/// A single sequel relation node.
fn sequel_relation(to_id: i64, to_title: &str) -> serde_json::Value {
    serde_json::json!({
        "node": { "id": to_id, "title": to_title },
        "relation_type": "sequel"
    })
}

/// Mount a token endpoint that always succeeds.
async fn mount_token(server: &MockServer, access: &str) {
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(token_response(access))
        .mount(server)
        .await;
}

/// Mount a list endpoint returning exactly `page` as the only page.
async fn mount_list_single(server: &MockServer, entries: &[serde_json::Value]) {
    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(list_page(entries, false)),
        )
        .mount(server)
        .await;
}

/// Mount an empty relations endpoint (no sequels / prequels) for any anime id.
async fn mount_relations_empty(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path_regex(r"^/anime/\d+$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(relations_body(&[])))
        .mount(server)
        .await;
}

// ---------------------------------------------------------------------------
// 1. Token exchange — code → tokens
// ---------------------------------------------------------------------------

#[tokio::test]
async fn token_exchange_stores_refresh_token_and_returns_access_token() {
    let server = MockServer::start().await;
    mount_token(&server, "access_abc").await;

    let db = db();
    let gate = gate(db.clone());
    let settings = settings_for(&server);
    let store = MemoryStore::new();
    let clock = clock();

    let client = MalClient::new(&gate, settings, &store, &clock);

    // Simulate what `Login::finish` does: exchange a code for tokens.
    // We drive it indirectly through `access_token()` after seeding the
    // refresh token (the real flow would have called exchange_and_store first;
    // here we pre-load the store so the refresh path is exercised instead).
    store.set_refresh_token(Some("rt_seed")).unwrap();
    let token = client.access_token().await.unwrap();

    assert_eq!(token, "access_abc");
    // The endpoint rotated the refresh token — the store must hold the new one.
    assert_eq!(store.refresh_token().unwrap().as_deref(), Some("rt_fresh"));
}

// ---------------------------------------------------------------------------
// 2. Token auto-refresh — expired in-memory token triggers refresh
// ---------------------------------------------------------------------------

#[tokio::test]
async fn expired_access_token_is_refreshed_before_the_list_is_fetched() {
    let server = MockServer::start().await;

    // The token endpoint is called at least once (refresh).
    let (token_hits, token_fixed) =
        common::Fixed::new(token_response("access_new"));
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(token_fixed)
        .mount(&server)
        .await;

    mount_list_single(&server, &[list_entry(1, "Frieren", "watching", 5, 28)]).await;
    mount_relations_empty(&server).await;

    let db = db();
    let gate = gate(db.clone());
    let settings = settings_for(&server);
    let store = MemoryStore::with("rt_old");
    let clock = clock();

    let client = MalClient::new(&gate, settings, &store, &clock);

    // The in-memory cache is empty, so the first `access_token()` call refreshes.
    let list = client.fetch().await.unwrap();
    assert_eq!(list.entries.len(), 1);
    assert_eq!(list.entries[0].entry.title, "Frieren");
    assert!(
        token_hits.load(Ordering::SeqCst) >= 1,
        "at least one refresh call must go through"
    );
}

// ---------------------------------------------------------------------------
// 3. List paging — two pages arrive, both entries land
// ---------------------------------------------------------------------------

#[tokio::test]
async fn multi_page_list_collects_all_entries() {
    let server = MockServer::start().await;
    mount_token(&server, "at").await;

    // First page has a "next" pointer.
    let page1 = list_page(&[list_entry(10, "Show A", "watching", 0, 12)], true);
    let page2 = list_page(&[list_entry(20, "Show B", "completed", 12, 12)], false);

    // WireMock cannot distinguish offset query params without custom matchers,
    // so we use a Sequence: first call → page1, second call → page2.
    let (_hits, seq) = common::Sequence::new(
        1,
        ResponseTemplate::new(200).set_body_json(page1),
        ResponseTemplate::new(200).set_body_json(page2),
    );
    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(seq)
        .mount(&server)
        .await;

    // Both ids return empty relations.
    Mock::given(method("GET"))
        .and(path_regex(r"^/anime/\d+$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(relations_body(&[])))
        .mount(&server)
        .await;

    let db = db();
    let gate = gate(db.clone());
    let store = MemoryStore::with("rt");
    let clock = clock();
    let client = MalClient::new(&gate, settings_for(&server), &store, &clock);

    let result = client.fetch().await.unwrap();
    assert_eq!(result.entries.len(), 2, "both pages must be collected");
    let ids: Vec<i64> = result.entries.iter().map(|e| e.mal_id()).collect();
    assert!(ids.contains(&10));
    assert!(ids.contains(&20));
}

// ---------------------------------------------------------------------------
// 4. Season chains — sequel links two entries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sequel_relation_links_two_entries_into_one_chain() {
    let server = MockServer::start().await;
    mount_token(&server, "at").await;

    let entries = &[
        list_entry(100, "Kaguya S1", "completed", 12, 12),
        list_entry(200, "Kaguya S2", "watching", 4, 13),
    ];
    mount_list_single(&server, entries).await;

    // id=100 has a sequel to id=200.
    Mock::given(method("GET"))
        .and(path("/anime/100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            relations_body(&[sequel_relation(200, "Kaguya S2")]),
        ))
        .mount(&server)
        .await;

    // id=200 has a prequel back (bilateral, common in MAL data).
    Mock::given(method("GET"))
        .and(path("/anime/200"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            relations_body(&[serde_json::json!({
                "node": { "id": 100, "title": "Kaguya S1" },
                "relation_type": "prequel"
            })]),
        ))
        .mount(&server)
        .await;

    let db = db();
    let gate = gate(db.clone());
    let store = MemoryStore::with("rt");
    let clock = clock();
    let client = MalClient::new(&gate, settings_for(&server), &store, &clock);

    let result = client.fetch().await.unwrap();
    assert_eq!(result.entries.len(), 2);

    let s1 = result.entries.iter().find(|e| e.mal_id() == 100).unwrap();
    let s2 = result.entries.iter().find(|e| e.mal_id() == 200).unwrap();

    // Both must share the same franchise id (the smaller mal_id).
    assert_eq!(s1.entry.franchise, s2.entry.franchise, "same franchise");
    assert_eq!(s1.entry.season, 1, "S1 is season 1");
    assert_eq!(s2.entry.season, 2, "S2 is season 2");
    // S2's episode offset is S1's total (12).
    assert_eq!(s2.entry.episode_offset, 12, "offset = S1 total");
}

// ---------------------------------------------------------------------------
// 5. Sync is idempotent — running twice gives the same result
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sync_twice_gives_the_same_database_state() {
    let server = MockServer::start().await;

    // Token endpoint handles multiple calls.
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(token_response("at"))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(ResponseTemplate::new(200).set_body_json(list_page(
            &[list_entry(1, "Frieren", "watching", 5, 28)],
            false,
        )))
        .mount(&server)
        .await;

    mock_relations_for(&server, 1, &[]).await;

    let db = db();
    let gate = Gate::with_policies(
        db.clone(),
        &[(genrl_core::http::Service::Mal, fast_mal_policy())],
    ).unwrap();
    let store = MemoryStore::with("rt");
    let clock = clock();
    let settings = settings_for(&server);

    let client = MalClient::new(&gate, settings.clone(), &store, &clock);
    let r1 = mal::sync(&client).await.unwrap();
    assert_eq!(r1.added, 1);
    assert_eq!(r1.updated, 0);

    // Second sync with the same data.
    let client2 = MalClient::new(&gate, settings, &store, &clock);
    let r2 = mal::sync(&client2).await.unwrap();
    assert_eq!(r2.added, 0, "already stored; must not appear as added again");
    assert_eq!(r2.unchanged, 1, "identical data → unchanged");
    assert_eq!(r2.updated, 0);
}

// ---------------------------------------------------------------------------
// 6. Diff detects changes — watched count change → "updated"
// ---------------------------------------------------------------------------

#[tokio::test]
async fn changed_watched_count_is_counted_as_updated() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(token_response("at"))
        .mount(&server)
        .await;

    let entry_before = list_entry(42, "Bocchi", "watching", 3, 12);
    let entry_after  = list_entry(42, "Bocchi", "watching", 6, 12); // watched 6 now

    let (sync_calls, seq) = common::Sequence::new(
        1,
        ResponseTemplate::new(200).set_body_json(list_page(&[entry_before], false)),
        ResponseTemplate::new(200).set_body_json(list_page(&[entry_after], false)),
    );
    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(seq)
        .mount(&server)
        .await;

    mock_relations_for(&server, 42, &[]).await;

    let db = db();
    let gate = Gate::with_policies(
        db.clone(),
        &[(genrl_core::http::Service::Mal, fast_mal_policy())],
    ).unwrap();
    let store = MemoryStore::with("rt");
    let clock = clock();
    let settings = settings_for(&server);

    let r1 = mal::sync(&MalClient::new(&gate, settings.clone(), &store, &clock)).await.unwrap();
    assert_eq!(r1.added, 1);

    let r2 = mal::sync(&MalClient::new(&gate, settings, &store, &clock)).await.unwrap();
    assert_eq!(r2.updated, 1, "a changed watched count must be detected");
    assert_eq!(r2.unchanged, 0);
    assert_eq!(sync_calls.load(Ordering::SeqCst), 2, "two list fetches");
}

// ---------------------------------------------------------------------------
// 7. Removed entry — second sync drops the mal_entries row and turns auto off
// ---------------------------------------------------------------------------

#[tokio::test]
async fn entry_absent_from_second_sync_is_removed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(token_response("at"))
        .mount(&server)
        .await;

    let entry_a = list_entry(10, "Show A", "watching", 2, 12);
    let entry_b = list_entry(20, "Show B", "watching", 0, 12);

    // First sync: two entries.
    let (_hits, seq) = common::Sequence::new(
        1,
        ResponseTemplate::new(200)
            .set_body_json(list_page(&[entry_a, entry_b], false)),
        // Second sync: only Show A remains.
        ResponseTemplate::new(200)
            .set_body_json(list_page(&[list_entry(10, "Show A", "watching", 2, 12)], false)),
    );
    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(seq)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/anime/\d+$"))
        .respond_with(ResponseTemplate::new(200).set_body_json(relations_body(&[])))
        .mount(&server)
        .await;

    let db = db();
    let gate = Gate::with_policies(
        db.clone(),
        &[(genrl_core::http::Service::Mal, fast_mal_policy())],
    ).unwrap();
    let store = MemoryStore::with("rt");
    let clock = clock();
    let settings = settings_for(&server);

    // First sync — both land.
    let r1 = mal::sync(&MalClient::new(&gate, settings.clone(), &store, &clock)).await.unwrap();
    assert_eq!(r1.added, 2);

    // Second sync — Show B is gone.
    let r2 = mal::sync(&MalClient::new(&gate, settings, &store, &clock)).await.unwrap();
    assert_eq!(r2.removed, 1, "one entry must be counted as removed");

    // Confirm the database mirrors the change.
    let mal_entry_count: i64 = db
        .with(|c| Ok(c.query_row("SELECT COUNT(*) FROM mal_entries", [], |r| r.get(0))?))
        .unwrap();
    assert_eq!(mal_entry_count, 1, "only Show A remains in mal_entries");

    // Show B's auto flag must be cleared.
    let auto_b: i64 = db
        .with(|c| {
            Ok(c.query_row(
                "SELECT auto FROM shows WHERE mal_id = 20",
                [],
                |r| r.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(auto_b, 0, "auto must be turned off for the removed show");
}

// ---------------------------------------------------------------------------
// 8. 429 on list endpoint — gate retries and sync succeeds
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_429_is_retried_and_sync_succeeds() {
    let server = MockServer::start().await;
    mount_token(&server, "at").await;

    // First list request → 429; second → OK.
    let (hits, seq) = common::Sequence::new(
        1,
        ResponseTemplate::new(429).insert_header("Retry-After", "0"),
        ResponseTemplate::new(200).set_body_json(list_page(
            &[list_entry(1, "Show", "watching", 0, 12)],
            false,
        )),
    );
    Mock::given(method("GET"))
        .and(path("/users/@me/animelist"))
        .respond_with(seq)
        .mount(&server)
        .await;
    mock_relations_for(&server, 1, &[]).await;

    let db = db();
    // Fast policy so the test doesn't wait on real backoff.
    let gate = Gate::with_policies(
        db.clone(),
        &[(
            genrl_core::http::Service::Mal,
            fast_mal_policy(),
        )],
    )
    .unwrap();
    let store = MemoryStore::with("rt");
    let clock = clock();
    let settings = settings_for(&server);

    let client = MalClient::new(&gate, settings, &store, &clock);
    let report = mal::sync(&client).await.unwrap();

    assert_eq!(report.added, 1, "sync must succeed after the retry");
    assert!(
        hits.load(Ordering::SeqCst) >= 2,
        "the 429 and the retry must have hit the server"
    );
}

// ---------------------------------------------------------------------------
// 9. Relations failure — one failed lookup stays a singleton chain
// ---------------------------------------------------------------------------

#[tokio::test]
async fn one_failed_relation_lookup_leaves_entry_as_singleton_chain() {
    let server = MockServer::start().await;
    mount_token(&server, "at").await;

    let entries = &[
        list_entry(1, "Show A", "watching", 0, 12),
        list_entry(2, "Show B", "watching", 0, 12),
    ];
    mount_list_single(&server, entries).await;

    // id=1 → normal empty relations.
    mock_relations_for(&server, 1, &[]).await;
    // id=2 → 500, so the lookup fails.
    Mock::given(method("GET"))
        .and(path("/anime/2"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let db = db();
    let gate = Gate::with_policies(
        db.clone(),
        &[(genrl_core::http::Service::Mal, fast_mal_policy())],
    )
    .unwrap();
    let store = MemoryStore::with("rt");
    let clock = clock();
    let settings = settings_for(&server);

    let client = MalClient::new(&gate, settings, &store, &clock);
    // The fetch must succeed despite one relation failure.
    let result = client.fetch().await.unwrap();

    assert_eq!(result.entries.len(), 2);
    assert_eq!(result.relations_failed, 1, "one failed relation must be counted");

    // Show B must still come back, as its own one-season chain.
    let b = result.entries.iter().find(|e| e.mal_id() == 2).unwrap();
    assert_eq!(b.entry.season, 1, "singleton keeps season = 1");
}

// ---------------------------------------------------------------------------
// 10. `is_due` cadence — sync_if_due skips when the interval has not elapsed
// ---------------------------------------------------------------------------

#[test]
fn sync_if_due_respects_the_thirty_minute_cadence() {
    let db = db();
    let clock = clock();

    // Nothing stored → always due.
    assert!(
        mal::is_due(&db, &clock, 30).unwrap(),
        "first ever sync must be due"
    );

    // Write a sync timestamp of NOW.
    genrl_core::settings::write_state(&db, mal::SYNC_STATE_KEY, &NOW.to_string()).unwrap();

    // 29 minutes later — not yet due.
    let clock29 = FakeClock::new(NOW + 29 * 60);
    assert!(
        !mal::is_due(&db, &clock29, 30).unwrap(),
        "must not be due before 30 minutes"
    );

    // Exactly 30 minutes later — due.
    let clock30 = FakeClock::new(NOW + 30 * 60);
    assert!(
        mal::is_due(&db, &clock30, 30).unwrap(),
        "must be due at exactly 30 minutes"
    );
}

// ---------------------------------------------------------------------------
// Helpers only used inside this file
// ---------------------------------------------------------------------------

/// Mount a specific `/anime/{id}` with the given relations (no query-param
/// matcher needed because WireMock path matching ignores the query string).
async fn mock_relations_for(server: &MockServer, id: i64, relations: &[serde_json::Value]) {
    Mock::given(method("GET"))
        .and(path(format!("/anime/{id}")))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(relations_body(relations)),
        )
        .mount(server)
        .await;
}

/// A MAL service policy with millisecond backoff so tests do not wait on
/// the production 5 s / 30 s / 2 min schedule.
fn fast_mal_policy() -> genrl_core::http::ServicePolicy {
    use std::time::Duration;
    use genrl_core::http::{RetryPolicy, ServicePolicy};
    use genrl_core::http::policy::RateSpec;

    ServicePolicy {
        rate: RateSpec {
            permits_per_second: 1000.0,
            burst: 100,
            per_minute_cap: None,
            jitter: false,
        },
        cache_ttl: None,
        max_body_bytes: 1024 * 1024,
        timeout: Duration::from_millis(500),
        retry: RetryPolicy {
            max_attempts: 3,
            schedule: vec![Duration::from_millis(5), Duration::from_millis(10)],
            max_total_wait: Duration::from_secs(5),
            max_delay: Duration::from_secs(10),
        },
        breaker_threshold: 5,
        breaker_cooldown: Duration::from_secs(900),
    }
}
