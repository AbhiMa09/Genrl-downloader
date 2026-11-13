use std::io::Write;
use std::process::{Command, Stdio};

use tempfile::TempDir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const CAPS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<caps>
  <server version="1.0" title="Prowlarr"/>
  <searching>
    <movie-search available="yes" supportedParams="q,imdbid,tmdbid"/>
    <tv-search available="yes" supportedParams="q,season,ep"/>
  </searching>
  <categories>
    <category id="2000" name="Movies">
      <subcat id="2040" name="Movies/HD"/>
    </category>
    <category id="5000" name="TV">
      <subcat id="5040" name="TV/HD"/>
    </category>
  </categories>
</caps>"#;

const KNOWN_FAKE_SECRET: &str = "SECRET_KEY_NEVER_LEAK_12345_XYZ";

fn genrl_bin() -> &'static str {
    env!("CARGO_BIN_EXE_genrl")
}

struct TestContext {
    temp_dir: TempDir,
}

impl TestContext {
    fn new() -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        // Run init
        let status = Command::new(genrl_bin())
            .arg("--data-dir")
            .arg(temp_dir.path())
            .arg("init")
            .status()
            .expect("run init");
        assert!(status.success(), "init must succeed");
        Self { temp_dir }
    }

    fn run(&self, args: &[&str]) -> (bool, String, String) {
        self.run_with_input_and_env(args, None, &[])
    }

    fn run_with_input_and_env(
        &self,
        args: &[&str],
        input: Option<&str>,
        env_vars: &[(&str, &str)],
    ) -> (bool, String, String) {
        let mut cmd = Command::new(genrl_bin());
        cmd.arg("--data-dir")
            .arg(self.temp_dir.path())
            .args(args)
            .env("GENRL_FORCE_NO_KEYRING", "1")
            .env(
                "GENRL_SECRETS_FILE",
                self.temp_dir.path().join("secrets.json"),
            )
            .env("XDG_CONFIG_HOME", self.temp_dir.path())
            .env("XDG_DATA_HOME", self.temp_dir.path());

        for (k, v) in env_vars {
            cmd.env(k, v);
        }

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("spawn command");
        if let Some(inp) = input {
            let mut stdin = child.stdin.take().expect("get stdin");
            stdin.write_all(inp.as_bytes()).expect("write stdin");
            drop(stdin);
        }

        let output = child.wait_with_output().expect("wait for output");
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (output.status.success(), stdout, stderr)
    }

    fn dump_db(&self) -> String {
        let db_path = self.temp_dir.path().join("genrl.db");
        let conn = rusqlite::Connection::open(db_path).unwrap();
        let mut dump = String::new();

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let tables: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        for table in tables {
            let mut tstmt = conn.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
            let count = tstmt.column_count();
            let mut rows = tstmt.query([]).unwrap();
            while let Some(row) = rows.next().unwrap() {
                for i in 0..count {
                    let val: rusqlite::types::Value = row.get(i).unwrap();
                    dump.push_str(&format!("{val:?} "));
                }
                dump.push('\n');
            }
        }
        dump
    }
}

#[test]
fn test_forbidden_apikey_flag_rejected() {
    let ctx = TestContext::new();

    // Test --apikey
    let (ok1, stdout1, stderr1) = ctx.run(&["indexers", "list", "--apikey", "secret123"]);
    assert!(!ok1, "must reject --apikey flag");
    assert!(
        stderr1.contains("API keys must never be passed as command-line arguments")
            || stdout1.contains("API keys must never be passed as command-line arguments"),
        "error must explain why: {stderr1}"
    );

    // Test --key
    let (ok2, stdout2, stderr2) = ctx.run(&["settings", "tmdb-status", "--key", "secret123"]);
    assert!(!ok2, "must reject --key flag");
    assert!(
        stderr2.contains("API keys must never be passed as command-line arguments")
            || stdout2.contains("API keys must never be passed as command-line arguments"),
        "error must explain why: {stderr2}"
    );
}

#[tokio::test]
async fn test_indexers_lifecycle() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_string(CAPS_XML))
        .mount(&mock)
        .await;

    let ctx = TestContext::new();

    // 1. Initially empty
    let (ok, out, _) = ctx.run(&["indexers", "list", "--json"]);
    assert!(ok);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);

    // 2. Add with invalid URL (fragment) rejected
    let (ok, _, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "prowlarr",
            "Prowlarr Main",
            "http://127.0.0.1:9696/api#frag",
        ],
        Some("my-api-key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("fragment"));

    // 3. Add with invalid URL (key in query) rejected
    let (ok, _, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "prowlarr",
            "Prowlarr Main",
            "http://127.0.0.1:9696/api?apikey=123",
        ],
        Some("my-api-key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("API key parameters"));

    // 4. Add valid indexer with --key-env
    let indexer_url = format!("{}/api/v2.0/indexers/test/results/torznab/api", mock.uri());
    let (ok, out, _) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "test_idx",
            "Test Indexer",
            &indexer_url,
            "--kinds",
            "movie,series",
            "--key-env",
            "MY_TEST_KEY_ENV",
            "--test",
            "--json",
        ],
        None,
        &[("MY_TEST_KEY_ENV", "test_key_val_999")],
    );
    assert!(ok, "add indexer must succeed: {out}");
    let added: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(added["id"], "test_idx");
    assert_eq!(added["name"], "Test Indexer");
    assert_eq!(added["has_key"], true);
    assert_eq!(added["last_test_status"], "ok");

    // 5. List shows indexer without leaking key
    let (ok, out, _) = ctx.run(&["indexers", "list", "--json"]);
    assert!(ok);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["id"], "test_idx");
    assert_eq!(list[0]["has_key"], true);
    assert!(list[0].get("api_key").is_none());

    // 6. Test command
    let (ok, out, _) = ctx.run(&["indexers", "test", "test_idx", "--json"]);
    assert!(ok);
    let test_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(test_res["ok"], true);
    assert_eq!(test_res["movie_search"], true);
    assert_eq!(test_res["tv_search"], true);
    assert_eq!(test_res["categories_count"], 4);

    // 7. Toggle enabled
    let (ok, out, err) = ctx.run(&["indexers", "set-enabled", "test_idx", "false", "--json"]);
    assert!(ok, "set-enabled failed: out={out}, err={err}");
    let toggle: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(toggle["enabled"], false);

    // 8. Remove
    let (ok, out, _) = ctx.run(&["indexers", "remove", "test_idx", "--json"]);
    assert!(ok);
    let rm: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(rm["ok"], true);

    // 9. List is empty again
    let (ok, out, _) = ctx.run(&["indexers", "list", "--json"]);
    assert!(ok);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_media_and_tmdb_lifecycle() {
    let mock = MockServer::start().await;

    // TMDB Movie search mock
    Mock::given(method("GET"))
        .and(path("/search/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page": 1,
            "results": [
                {
                    "id": 693134,
                    "title": "Dune: Part Two",
                    "original_title": "Dune: Part Two",
                    "release_date": "2024-03-01",
                    "poster_path": "/dune.jpg",
                    "overview": "Paul Atreides unites with Chani and the Fremen while seeking revenge."
                }
            ],
            "total_results": 1
        })))
        .mount(&mock)
        .await;

    // TMDB Movie details mock
    Mock::given(method("GET"))
        .and(path("/movie/693134"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 693134,
            "title": "Dune: Part Two",
            "original_title": "Dune: Part Two",
            "release_date": "2024-03-01",
            "poster_path": "/dune.jpg",
            "overview": "Paul Atreides unites with Chani and the Fremen while seeking revenge."
        })))
        .mount(&mock)
        .await;

    // TMDB TV details mock
    Mock::given(method("GET"))
        .and(path("/tv/1399"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 1399,
            "name": "Game of Thrones",
            "original_name": "Game of Thrones",
            "first_air_date": "2011-04-17",
            "poster_path": "/got.jpg",
            "status": "Ended",
            "seasons": [
                {
                    "id": 3624,
                    "season_number": 0,
                    "name": "Specials",
                    "episode_count": 5
                },
                {
                    "id": 3625,
                    "season_number": 1,
                    "name": "Season 1",
                    "episode_count": 10
                }
            ]
        })))
        .mount(&mock)
        .await;

    // TMDB Season 1 episodes mock
    Mock::given(method("GET"))
        .and(path("/tv/1399/season/1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 3625,
            "season_number": 1,
            "episodes": [
                {
                    "id": 63056,
                    "season_number": 1,
                    "episode_number": 1,
                    "name": "Winter Is Coming",
                    "air_date": "2011-04-17"
                },
                {
                    "id": 63057,
                    "season_number": 1,
                    "episode_number": 2,
                    "name": "The Kingsroad",
                    "air_date": "2011-04-24"
                }
            ]
        })))
        .mount(&mock)
        .await;

    let ctx = TestContext::new();

    // 1. TMDB Status before config -> configured: false
    let (ok, out, _) = ctx.run(&["settings", "tmdb-status", "--json"]);
    assert!(ok);
    let st: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(st["configured"], false);

    // 2. Media search without TMDB key -> tmdb_not_configured
    let (ok, out, _) = ctx.run(&["media", "search", "Dune", "--json"]);
    assert!(ok);
    let err_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(err_res["error"], "tmdb_not_configured");

    // 3. Configure TMDB key via --key-env
    let (ok, _, _) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key", "--key-env", "MY_TMDB_ENV"],
        None,
        &[("MY_TMDB_ENV", "tmdb_secret_key_123")],
    );
    assert!(ok);

    // 4. TMDB status is now configured
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["settings", "tmdb-status", "--json"],
        None,
        &[("GENRL_TMDB_KEY", "tmdb_secret_key_123")],
    );
    assert!(ok);
    let st: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(st["configured"], true);

    // 5. Test TMDB connectivity
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["settings", "test-tmdb", "--json"],
        None,
        &[
            ("GENRL_TMDB_KEY", "tmdb_secret_key_123"),
            ("GENRL_TMDB_BASE_URL", &mock.uri()),
        ],
    );
    assert!(ok);
    let test_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(test_res["ok"], true);

    // 6. Media search with mock TMDB
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["media", "search", "Dune", "--kind", "movie", "--json"],
        None,
        &[
            ("GENRL_TMDB_KEY", "tmdb_secret_key_123"),
            ("GENRL_TMDB_BASE_URL", &mock.uri()),
        ],
    );
    assert!(ok, "media search: {out}");
    let search_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(search_res.as_array().unwrap().len(), 1);
    assert_eq!(search_res[0]["tmdb_id"], 693134);
    assert_eq!(search_res[0]["title"], "Dune: Part Two");
    assert_eq!(search_res[0]["year"], 2024);

    // 7. Media add Movie (First call -> jobs_created = 1)
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["media", "add", "693134", "--kind", "movie", "--json"],
        None,
        &[
            ("GENRL_TMDB_KEY", "tmdb_secret_key_123"),
            ("GENRL_TMDB_BASE_URL", &mock.uri()),
        ],
    );
    assert!(ok);
    let add_res: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(add_res["jobs_created"], 1);
    let movie_media_id = add_res["media_id"].as_i64().unwrap();

    // 8. IDEMPOTENCY: Media add Movie second call -> jobs_created = 0, same media_id
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["media", "add", "693134", "--kind", "movie", "--json"],
        None,
        &[
            ("GENRL_TMDB_KEY", "tmdb_secret_key_123"),
            ("GENRL_TMDB_BASE_URL", &mock.uri()),
        ],
    );
    assert!(ok);
    let add_res2: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(add_res2["media_id"], movie_media_id);
    assert_eq!(add_res2["jobs_created"], 0);

    // 9. Media add Series (Season 1 with 2 episodes, specials ignored)
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["media", "add", "1399", "--kind", "series", "--json"],
        None,
        &[
            ("GENRL_TMDB_KEY", "tmdb_secret_key_123"),
            ("GENRL_TMDB_BASE_URL", &mock.uri()),
        ],
    );
    assert!(ok);
    let series_add: serde_json::Value = serde_json::from_str(&out).unwrap();
    let series_media_id = series_add["media_id"].as_i64().unwrap();
    assert!(series_add["jobs_created"].as_u64().unwrap() > 0);

    // 10. Media list displays both movie and series with progress
    let (ok, out, _) = ctx.run(&["media", "list", "--json"]);
    assert!(ok);
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2);

    let series_item = list
        .as_array()
        .unwrap()
        .iter()
        .find(|it| it["id"] == series_media_id)
        .unwrap();
    assert_eq!(series_item["title"], "Game of Thrones");
    assert!(series_item.get("progress").is_some());

    // 11. Media remove
    let (ok, out, _) = ctx.run(&["media", "remove", &movie_media_id.to_string(), "--json"]);
    assert!(ok);
    let rm: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(rm["ok"], true);

    let (ok, out, _) = ctx.run(&["media", "list", "--kind", "movie", "--json"]);
    assert!(ok);
    let list_after: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list_after.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_secret_is_never_leaked_in_stdout_stderr_db_or_events() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(query_param("t", "caps"))
        .respond_with(ResponseTemplate::new(200).set_body_string(CAPS_XML))
        .mount(&mock)
        .await;

    Mock::given(method("GET"))
        .and(path("/search/movie"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "page": 1,
            "results": [],
            "total_results": 0
        })))
        .mount(&mock)
        .await;

    let ctx = TestContext::new();

    let mut all_stdout = String::new();
    let mut all_stderr = String::new();

    // 1. Add indexer using KNOWN_FAKE_SECRET via stdin
    let indexer_url = format!("{}/api/v2.0/indexers/test/results/torznab/api", mock.uri());
    let (_, out, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "sec_idx",
            "Secret Indexer",
            &indexer_url,
            "--test",
            "--json",
        ],
        Some(&format!("{KNOWN_FAKE_SECRET}\n")),
        &[],
    );
    all_stdout.push_str(&out);
    all_stderr.push_str(&err);

    // 2. Set TMDB key using KNOWN_FAKE_SECRET via stdin
    let (_, out, err) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key"],
        Some(&format!("{KNOWN_FAKE_SECRET}\n")),
        &[],
    );
    all_stdout.push_str(&out);
    all_stderr.push_str(&err);

    // 3. Run indexers list
    let (_, out, err) = ctx.run(&["indexers", "list", "--json"]);
    all_stdout.push_str(&out);
    all_stderr.push_str(&err);

    // 4. Run doctor
    let (_, out, err) = ctx.run(&["doctor"]);
    all_stdout.push_str(&out);
    all_stderr.push_str(&err);
    assert!(out.contains("Torznab    : ok (1 indexer)"));
    assert!(out.contains("TMDB       : ok"));

    // 5. Run media search
    let (_, out, err) = ctx.run_with_input_and_env(
        &["media", "search", "Test", "--json"],
        None,
        &[("GENRL_TMDB_BASE_URL", &mock.uri())],
    );
    all_stdout.push_str(&out);
    all_stderr.push_str(&err);

    // ASSERT: secret is NEVER in stdout or stderr
    assert!(
        !all_stdout.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in stdout!"
    );
    assert!(
        !all_stderr.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in stderr!"
    );

    // ASSERT: secret is NEVER in SQLite database dump
    let db_dump = ctx.dump_db();
    assert!(
        !db_dump.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in SQLite database!"
    );

    // ASSERT: events query has no secret
    let (_, events_out, _) = ctx.run(&["events", "--limit", "100", "--json"]);
    assert!(
        !events_out.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in event log!"
    );
}

#[test]
fn test_contract_generation_deterministic() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();

    let output1 = Command::new("python3")
        .args(["tools/generate_contract.py", "--check"])
        .current_dir(&repo_root)
        .output()
        .expect("run generate_contract.py run 1");
    assert!(
        output1.status.success(),
        "run 1 must succeed: {}",
        String::from_utf8_lossy(&output1.stderr)
    );

    let output2 = Command::new("python3")
        .args(["tools/generate_contract.py", "--check"])
        .current_dir(&repo_root)
        .output()
        .expect("run generate_contract.py run 2");
    assert!(
        output2.status.success(),
        "run 2 must succeed: {}",
        String::from_utf8_lossy(&output2.stderr)
    );
}
