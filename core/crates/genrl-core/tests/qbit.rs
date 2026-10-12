//! The qBittorrent client against a mock WebUI: session handling, the 403
//! re-login, the 4.x/5.x control paths, and clear errors when it is down.

mod common;

use common::{Sequence, Fixed};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use genrl_core::qbit::{magnet_info_hash, ApiGen, Qbit};
use genrl_core::{Db, Gate};

fn torrent_json(hash: &str, progress: f64) -> String {
    format!(
        r#"[{{
            "hash": "{hash}",
            "name": "[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv",
            "progress": {progress},
            "dlspeed": 1048576,
            "upspeed": 0,
            "eta": 42,
            "num_seeds": 12,
            "num_leechs": 3,
            "size": 1600000000,
            "state": "downloading",
            "save_path": "/library/",
            "content_path": "/library/Show/01.mkv",
            "amount_left": 800000000,
            "added_on": 1789227900,
            "completion_on": 0,
            "category": "genrl"
        }}]"#
    )
}

fn gate() -> Gate {
    Gate::new(Db::open_in_memory().unwrap()).unwrap()
}

async fn mount_login(server: &MockServer, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(response)
        .mount(server)
        .await;
}

fn login_ok() -> ResponseTemplate {
    ResponseTemplate::new(200)
        .set_body_string("Ok.")
        .insert_header("Set-Cookie", "SID=testsess; path=/")
}

/// What qBittorrent 5.2.4 actually answers: `204`, no body at all, and a
/// port-suffixed cookie instead of `SID`.
fn login_ok_5_2() -> ResponseTemplate {
    ResponseTemplate::new(204).insert_header(
        "Set-Cookie",
        "QBT_SID_8080=sessiontoken; HttpOnly; SameSite=Lax; path=/",
    )
}

#[tokio::test]
async fn first_call_logs_in_then_carries_the_session_cookie() {
    let server = MockServer::start().await;
    mount_login(&server, login_ok()).await;

    // The API rejects the cookie-less first request exactly once.
    let (hits, sequence) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200).set_body_string(torrent_json("abc123", 0.5)),
    );
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(sequence)
        .mount(&server)
        .await;

    let qbit = Qbit::new(&server.uri(), "admin", Some("adminadmin".into()));
    let torrents = qbit.list(&gate(), None).await.unwrap();

    assert_eq!(torrents.len(), 1);
    assert_eq!(torrents[0].hash, "abc123");
    assert_eq!(torrents[0].progress, 0.5);
    assert!(torrents[0].content_path.starts_with('/'));
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2, "403 then retry");

    // Exactly one login, and the retried request carried the SID.
    let requests = server.received_requests().await.unwrap();
    let logins = requests
        .iter()
        .filter(|r| r.url.path() == "/api/v2/auth/login")
        .count();
    assert_eq!(logins, 1, "session is reused, not re-established per call");

    let info_calls: Vec<_> = requests
        .iter()
        .filter(|r| r.url.path() == "/api/v2/torrents/info")
        .collect();
    let first_cookie = header(&info_calls[0].headers, "cookie");
    let second_cookie = header(&info_calls[1].headers, "cookie");
    assert!(first_cookie.is_none(), "first call had no session yet");
    assert_eq!(second_cookie.as_deref(), Some("SID=testsess"));
}

fn header(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

#[tokio::test]
async fn add_sends_magnets_and_save_path_as_a_form() {
    let server = MockServer::start().await;
    mount_login(&server, login_ok()).await;

    let (hits, sequence) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200),
    );
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(sequence)
        .mount(&server)
        .await;

    let magnet = format!(
        "magnet:?xt=urn:btih:{}&dn=Show",
        "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b"
    );
    let qbit = Qbit::new(&server.uri(), "admin", Some("pw".into()));
    qbit.add(&gate(), &[magnet], Some("/downloads/anime"), Some("genrl"))
        .await
        .unwrap();

    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2);

    let requests = server.received_requests().await.unwrap();
    let add = requests
        .iter()
        .find(|r| r.url.path() == "/api/v2/torrents/add")
        .expect("add was called");
    // The second (authenticated) attempt is the one that must carry the form.
    let body = String::from_utf8_lossy(&add.body);
    assert!(body.contains("urls=magnet%3A%3Fxt%3Durn%3Abtih%3A"), "{body}");
    assert!(body.contains("savepath=%2Fdownloads%2Fanime"), "{body}");
    assert!(body.contains("category=genrl"), "{body}");
}

#[tokio::test]
async fn control_paths_depend_on_the_api_generation() {
    // --- qBittorrent 5.x: stop/start -----------------------------------
    let server = MockServer::start().await;
    mount_login(&server, login_ok()).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v5.2.4"))
        .mount(&server)
        .await;
    let (hits5, sequence5) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200),
    );
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/stop"))
        .respond_with(sequence5)
        .mount(&server)
        .await;

    let v5 = Qbit::new(&server.uri(), "admin", None);
    let health = v5.health(&gate()).await.unwrap();
    assert_eq!(health.gen, ApiGen::V5);
    assert_eq!(health.version, "v5.2.4");
    v5.set_active(&gate(), "abc123", false).await.unwrap();
    assert_eq!(hits5.load(std::sync::atomic::Ordering::SeqCst), 2);
    drop(v5);

    // --- qBittorrent 4.x: pause/resume ---------------------------------
    let v4_server = MockServer::start().await;
    mount_login(&v4_server, login_ok()).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v4.6.4"))
        .mount(&v4_server)
        .await;
    let (hits4, sequence4) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200),
    );
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/pause"))
        .respond_with(sequence4)
        .mount(&v4_server)
        .await;

    let v4 = Qbit::new(&v4_server.uri(), "admin", None);
    v4.set_active(&gate(), "abc123", false).await.unwrap();
    assert_eq!(hits4.load(std::sync::atomic::Ordering::SeqCst), 2);

    let requests = v4_server.received_requests().await.unwrap();
    assert!(
        requests.iter().any(|r| r.url.path() == "/api/v2/torrents/pause"),
        "4.x uses pause"
    );
}

#[tokio::test]
async fn the_5_2_cookie_dance_is_carried_back_verbatim() {
    let server = MockServer::start().await;
    mount_login(&server, login_ok_5_2()).await;

    let (hits, sequence) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200).set_body_string(torrent_json("abc123", 0.75)),
    );
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(sequence)
        .mount(&server)
        .await;

    let qbit = Qbit::new(&server.uri(), "admin", Some("pw".into()));
    let torrents = qbit.list(&gate(), None).await.unwrap();
    assert_eq!(torrents.len(), 1);
    assert_eq!(torrents[0].progress, 0.75);
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2);

    // The port-suffixed cookie — not `SID=` — is what goes back on the wire.
    let requests = server.received_requests().await.unwrap();
    let info_calls: Vec<_> = requests
        .iter()
        .filter(|r| r.url.path() == "/api/v2/torrents/info")
        .collect();
    assert_eq!(
        header(&info_calls[1].headers, "cookie").as_deref(),
        Some("QBT_SID_8080=sessiontoken")
    );
}

#[tokio::test]
async fn a_401_login_is_an_auth_error_with_the_fix() {
    let server = MockServer::start().await;
    mount_login(&server, ResponseTemplate::new(401).set_body_string("Unauthorized")).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let qbit = Qbit::new(&server.uri(), "admin", Some("nope".into()));
    let err = qbit.list(&gate(), None).await.unwrap_err();

    let message = err.to_string();
    assert!(message.contains("authentication failed"), "{message}");
    assert!(message.contains("HTTP 401"), "the status is named: {message}");
    assert!(message.contains("genrl qbit password"), "{message}");
    assert!(!message.contains("nope"), "the password is never echoed");
}

#[tokio::test]
async fn a_rejected_login_names_the_way_to_fix_it() {
    let server = MockServer::start().await;
    mount_login(&server, ResponseTemplate::new(200).set_body_string("Fails.")).await;
    Mock::given(method("GET"))
        .and(path("/api/v2/torrents/info"))
        .respond_with(ResponseTemplate::new(403))
        .mount(&server)
        .await;

    let qbit = Qbit::new(&server.uri(), "admin", Some("wrong".into()));
    let err = qbit.list(&gate(), None).await.unwrap_err();

    let message = err.to_string();
    assert!(message.contains("authentication failed"), "{message}");
    assert!(
        message.contains("genrl qbit password") || message.contains("GENRL_QBIT_PASSWORD"),
        "the error must say how to set the password: {message}"
    );
    assert!(!message.contains("wrong"), "the password itself is never echoed");
}

#[tokio::test]
async fn an_unreachable_webui_is_a_connection_failure() {
    // Bind, then release a port so nothing is listening on it.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let qbit = Qbit::new(&format!("http://127.0.0.1:{port}"), "admin", None);
    let err = qbit.health(&gate()).await.unwrap_err();
    assert!(
        err.is_connection_failure(),
        "health check must recognise 'not running': {err:?}"
    );
    assert!(err.is_transient(), "the job queue will retry it later");
}

#[tokio::test]
async fn remove_and_speeds_go_through_the_gate() {
    let server = MockServer::start().await;
    mount_login(&server, login_ok()).await;

    let (hits, sequence) = Sequence::new(
        1,
        ResponseTemplate::new(403),
        ResponseTemplate::new(200),
    );
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/delete"))
        .respond_with(sequence)
        .mount(&server)
        .await;
    let (speed_hits, speed_fixed) = Fixed::new(ResponseTemplate::new(200).set_body_string(
        r#"{"dl_info_speed": 5242880, "up_info_speed": 1024, "connection_status": "connected"}"#,
    ));
    Mock::given(method("GET"))
        .and(path("/api/v2/transfer/info"))
        .respond_with(speed_fixed)
        .mount(&server)
        .await;

    let qbit = Qbit::new(&server.uri(), "admin", Some("pw".into()));
    qbit.remove(&gate(), "abc123", false).await.unwrap();
    let speeds = qbit.speeds(&gate()).await.unwrap();
    assert_eq!(speeds.download, 5 * 1024 * 1024);
    assert_eq!(speeds.upload, 1024);
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(speed_hits.load(std::sync::atomic::Ordering::SeqCst), 1);

    let requests = server.received_requests().await.unwrap();
    let delete = requests
        .iter()
        .find(|r| r.url.path() == "/api/v2/torrents/delete")
        .unwrap();
    let body = String::from_utf8_lossy(&delete.body);
    assert!(body.contains("hashes=ABC123"), "hashes are upper-cased: {body}");
    assert!(body.contains("deleteFiles=false"), "{body}");
}

#[test]
fn magnets_are_the_only_link_that_yields_a_hash_here() {
    let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert_eq!(
        magnet_info_hash(&format!("magnet:?xt=urn:btih:{hash}")).as_deref(),
        Some(hash)
    );
    assert_eq!(magnet_info_hash("https://nyaa.si/download/1.torrent"), None);
}
