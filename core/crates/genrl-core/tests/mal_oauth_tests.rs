//! MAL OAuth Tests: state mismatch, denied/cancel, timeout, port fallback,
//! refresh rotation, and no-keyring path.
//! All external calls run against local mocks (labeled mock).

mod common;

use std::time::Duration;
use tokio::io::AsyncWriteExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::clock::FakeClock;
use genrl_core::mal::{self, MalClient, MemoryStore, TokenStore};
use genrl_core::settings::Settings;
use genrl_core::{Db, Gate};

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

fn settings_for(server: &MockServer) -> Settings {
    Settings {
        mal_base_url: server.uri(),
        mal_client_id: "fixture-client".into(),
        ..Settings::default()
    }
}

#[tokio::test]
async fn test_oauth_state_mismatch() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let req =
            "GET /callback?code=mock_code&state=wrong_state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let _ = stream.write_all(req.as_bytes()).await;
    });

    let res = mal::redirect_code(listener, "expected_state", Duration::from_millis(500)).await;
    let _ = client_task.await;
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("the redirect state did not match"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_oauth_denied_cancel() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let client_task = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(20)).await;
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let req = "GET /callback?error=access_denied&state=expected_state HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        let _ = stream.write_all(req.as_bytes()).await;
    });

    let res = mal::redirect_code(listener, "expected_state", Duration::from_millis(500)).await;
    let _ = client_task.await;
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("MyAnimeList denied the login: access_denied"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_oauth_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let res = mal::redirect_code(listener, "expected_state", Duration::from_millis(50)).await;
    assert!(res.is_err());
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("the login timed out waiting for the browser redirect"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn test_oauth_busy_port_is_a_clear_error_not_a_fallback() {
    let settings = Settings::default();

    // MAL only accepts the one registered redirect, so a busy port must fail
    // up front rather than send the user to a login MAL would reject.
    let guard = tokio::net::TcpListener::bind(("127.0.0.1", 21445)).await;
    if let Ok(_busy) = guard {
        let err = match mal::start_login(&settings).await {
            Err(e) => e.to_string(),
            Ok(_) => panic!("expected an error while the redirect port is busy"),
        };
        assert!(
            err.contains("redirect port 21445 on 127.0.0.1 is busy"),
            "unexpected error: {err}"
        );
    }
}

#[tokio::test]
async fn test_oauth_refresh_rotation() {
    let server = MockServer::start().await; // Labeled mock
    let db = db();
    let gate = gate(db);
    let clock = clock();
    let store = MemoryStore::with("mock-refresh-token-1");
    let settings = settings_for(&server);

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock-access-token-1",
            "refresh_token": "mock-rotated-refresh-token-2",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    let client = MalClient::new(&gate, settings, &store, &clock);
    let access = client.access_token().await.unwrap();
    assert_eq!(access, "mock-access-token-1");

    // Assert refresh token was rotated in the store
    assert_eq!(
        store.refresh_token().unwrap().as_deref(),
        Some("mock-rotated-refresh-token-2")
    );
}

#[tokio::test]
async fn test_oauth_no_keyring_session_path() {
    let server = MockServer::start().await; // Labeled mock
    let db = db();
    let gate = gate(db);
    let clock = clock();
    let store = MemoryStore::new(); // In-memory session store (no keyring)
    let settings = settings_for(&server);

    // Mock token exchange
    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock-session-access",
            "refresh_token": "mock-session-refresh",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    // Mock account endpoint
    Mock::given(method("GET"))
        .and(path("/users/@me"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 99999,
            "name": "mock_guest_user"
        })))
        .mount(&server)
        .await;

    let client = MalClient::new(&gate, settings, &store, &clock);
    let tokens = client
        .exchange_and_store(
            "http://127.0.0.1:21445/callback",
            "mock_code",
            "mock_verifier",
        )
        .await
        .unwrap();
    assert_eq!(tokens.access_token, "mock-session-access");
    assert_eq!(
        store.refresh_token().unwrap().as_deref(),
        Some("mock-session-refresh")
    );

    let account = client.account().await.unwrap();
    assert_eq!(account.name, "mock_guest_user");
}

#[tokio::test]
async fn test_default_client_id_used_without_secret() {
    let server = MockServer::start().await; // Labeled mock
    let db = db();
    let gate = gate(db);
    let clock = clock();
    let store = MemoryStore::with("mock-refresh");
    // Settings has empty client_id -> falls back to DEFAULT_MAL_CLIENT_ID
    let settings = Settings {
        mal_base_url: server.uri(),
        mal_client_id: String::new(),
        ..Settings::default()
    };

    // Ensure GENRL_MAL_CLIENT_SECRET is ignored when using default client
    std::env::set_var("GENRL_MAL_CLIENT_SECRET", "leak_test_secret");

    Mock::given(method("POST"))
        .and(path("/oauth2/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mock-default-access",
            "refresh_token": "mock-rotated-refresh",
            "expires_in": 3600
        })))
        .mount(&server)
        .await;

    let client = MalClient::new(&gate, settings, &store, &clock);
    let token = client.access_token().await.unwrap();
    assert_eq!(token, "mock-default-access");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let req_body = String::from_utf8_lossy(&requests[0].body);
    // Verified: client_id matches DEFAULT_MAL_CLIENT_ID and no client_secret is sent
    assert!(req_body.contains(&format!("client_id={}", mal::DEFAULT_MAL_CLIENT_ID)));
    assert!(!req_body.contains("client_secret"));

    std::env::remove_var("GENRL_MAL_CLIENT_SECRET");
}
