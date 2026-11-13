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
    assert!(out.contains("Torznab    : configured, last test ok (1 indexer)"));
    assert!(out.contains("TMDB       : configured, last test never"));

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

#[test]
fn test_stdin_path_and_key_env_and_trimming() {
    let ctx = TestContext::new();

    // 1. Empty stdin gives a clear error
    let (ok, _out, err) =
        ctx.run_with_input_and_env(&["settings", "set-tmdb-key"], Some("   \n"), &[]);
    assert!(!ok);
    assert!(err.contains("No API key provided on stdin"));

    let (ok, _out, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx_empty",
            "Indexer Empty",
            "https://indexer.example.com",
        ],
        Some(""),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("No API key provided on stdin"));

    // 2. Trailing newline and spaces trimmed for settings set-tmdb-key
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key"],
        Some("  my_tmdb_trimmed_key  \r\n"),
        &[],
    );
    assert!(ok);
    assert!(out.contains("TMDB API key stored successfully"));

    // Verify secrets.json has the trimmed key without whitespace
    let secrets_file = ctx.temp_dir.path().join("secrets.json");
    let secrets_content = std::fs::read_to_string(&secrets_file).unwrap();
    assert!(secrets_content.contains("\"indexer_key_tmdb\": \"my_tmdb_trimmed_key\""));

    // 3. Trailing newline and spaces trimmed for indexers add via stdin
    let (ok, out, _) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx_trim",
            "Indexer Trim",
            "https://indexer.example.com",
        ],
        Some("  my_indexer_trimmed_key  \n"),
        &[],
    );
    assert!(ok);
    assert!(out.contains("Indexer added: Indexer Trim (idx_trim)"));

    let secrets_content = std::fs::read_to_string(&secrets_file).unwrap();
    assert!(secrets_content.contains("\"indexer_key_idx_trim\": \"my_indexer_trimmed_key\""));

    // 4. Test --key-env with valid, missing, and whitespace env var
    let (ok, out, _) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key", "--key-env", "VALID_TMDB_ENV"],
        None,
        &[("VALID_TMDB_ENV", "  env_tmdb_key_val  \n")],
    );
    assert!(ok);
    assert!(out.contains("TMDB API key stored successfully"));
    let secrets_content = std::fs::read_to_string(&secrets_file).unwrap();
    assert!(secrets_content.contains("\"indexer_key_tmdb\": \"env_tmdb_key_val\""));

    let (ok, out, _) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx_env",
            "Indexer Env",
            "https://indexer.example.com",
            "--key-env",
            "VALID_IDX_ENV",
        ],
        None,
        &[("VALID_IDX_ENV", "  env_idx_key_val  ")],
    );
    assert!(ok);
    assert!(out.contains("Indexer added: Indexer Env (idx_env)"));
    let secrets_content = std::fs::read_to_string(&secrets_file).unwrap();
    assert!(secrets_content.contains("\"indexer_key_idx_env\": \"env_idx_key_val\""));

    // Missing env var error
    let (ok, _out, err) = ctx.run(&["settings", "set-tmdb-key", "--key-env", "NONEXISTENT_VAR"]);
    assert!(!ok);
    assert!(err.contains("is not set or empty"));

    // Empty/whitespace env var error
    let (ok, _out, err) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key", "--key-env", "EMPTY_VAR"],
        None,
        &[("EMPTY_VAR", "   \t\n")],
    );
    assert!(!ok);
    assert!(err.contains("contains an empty API key"));
}

#[tokio::test]
async fn test_doctor_wording() {
    let ctx = TestContext::new();

    // 1. Unconfigured state
    let (_, out, _) = ctx.run(&["doctor"]);
    assert!(out.contains("Torznab    : not configured"));
    assert!(out.contains("TMDB       : not configured"));

    // 2. Add an indexer without testing -> "configured, last test never (1 indexer)"
    ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "doc_idx",
            "Doctor Idx",
            "https://doc.example.com",
        ],
        Some("test_key\n"),
        &[],
    );
    let (_, out, _) = ctx.run(&["doctor"]);
    assert!(out.contains("Torznab    : configured, last test never (1 indexer)"));

    // 3. Set TMDB key without testing -> "configured, last test never"
    ctx.run_with_input_and_env(&["settings", "set-tmdb-key"], Some("tmdb_key\n"), &[]);
    let (_, out, _) = ctx.run(&["doctor"]);
    assert!(out.contains("Torznab    : configured, last test never (1 indexer)"));
    assert!(out.contains("TMDB       : configured, last test never"));

    // 4. Test indexer with wiremock server responding with caps
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::query_param("t", "caps"))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
            r#"<caps><server version="1.0" title="mock"/><searching><movie-search available="yes"/></searching><categories><category id="2000" name="Movies"/></categories></caps>"#,
        ))
        .mount(&mock)
        .await;

    // Remove old indexer and add with mock uri and --test
    ctx.run(&["indexers", "remove", "doc_idx"]);
    let (ok, _, _) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "doc_idx",
            "Doctor Idx",
            &mock.uri(),
            "--test",
        ],
        Some("test_key\n"),
        &[],
    );
    assert!(ok);

    let (_, out, _) = ctx.run(&["doctor"]);
    assert!(out.contains("Torznab    : configured, last test ok (1 indexer)"));

    // 5. Test TMDB with wiremock server responding OK
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/search/movie"))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "page": 1,
                "results": [],
                "total_pages": 1,
                "total_results": 0
            })),
        )
        .mount(&mock)
        .await;

    let (ok, _, _) = ctx.run_with_input_and_env(
        &["settings", "test-tmdb"],
        None,
        &[("GENRL_TMDB_BASE_URL", &mock.uri())],
    );
    assert!(ok);

    let (_, out, _) = ctx.run(&["doctor"]);
    assert!(out.contains("TMDB       : configured, last test ok"));
}

#[test]
fn test_indexers_add_url_validation_rules() {
    let ctx = TestContext::new();

    // 1. userinfo (username / password in URL)
    let (ok, _, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx1",
            "Name",
            "http://user:pass@example.com/api",
        ],
        Some("key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("userinfo"));

    // 2. URL fragment (#)
    let (ok, _, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx2",
            "Name",
            "http://example.com/api#fragment",
        ],
        Some("key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("fragment"));

    // 3. Query string containing key-like parameter
    let (ok, _, err) = ctx.run_with_input_and_env(
        &[
            "indexers",
            "add",
            "idx3",
            "Name",
            "http://example.com/api?apikey=12345",
        ],
        Some("key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("API key parameters in query string"));

    // 4. Non-http scheme (ftp)
    let (ok, _, err) = ctx.run_with_input_and_env(
        &["indexers", "add", "idx4", "Name", "ftp://example.com/api"],
        Some("key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("http or https scheme"));

    // 5. Length over 2048 chars
    let long_path = "a".repeat(2050);
    let long_url = format!("http://example.com/{long_path}");
    let (ok, _, err) = ctx.run_with_input_and_env(
        &["indexers", "add", "idx5", "Name", &long_url],
        Some("key\n"),
        &[],
    );
    assert!(!ok);
    assert!(err.contains("exceeds maximum length of 2048 characters"));
}

#[test]
fn test_media_search_without_tmdb_key_returns_tmdb_not_configured() {
    let ctx = TestContext::new();

    // 1. Text mode
    let (ok, out, err) = ctx.run(&["media", "search", "Dune"]);
    assert!(!ok);
    let combined = format!("{out}\n{err}");
    assert!(combined.contains("tmdb_not_configured"));

    // 2. JSON mode
    let (ok, out, _err) = ctx.run(&["media", "search", "Dune", "--json"]);
    assert!(ok); // In JSON mode it prints JSON error payload
    let v: serde_json::Value = serde_json::from_str(&out).expect("valid json output");
    assert_eq!(v["error"], "tmdb_not_configured");
}

#[tokio::test]
async fn test_media_list_poster_path_and_overview_fields() {
    let mock = MockServer::start().await;

    // Movie details with an overview longer than the 1000-char storage cap.
    let long_overview = "A".repeat(1500);
    Mock::given(method("GET"))
        .and(path("/movie/693134"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": 693134,
            "title": "Dune: Part Two",
            "original_title": "Dune: Part Two",
            "release_date": "2024-03-01",
            "poster_path": "/dune.jpg",
            "overview": long_overview
        })))
        .mount(&mock)
        .await;

    let ctx = TestContext::new();

    // Configure the TMDB key via stdin with the known fake secret (never as an argument).
    let (ok, _, _) = ctx.run_with_input_and_env(
        &["settings", "set-tmdb-key"],
        Some(&format!("{KNOWN_FAKE_SECRET}\n")),
        &[],
    );
    assert!(ok);

    // media add stores only {"overview": ...} in raw_json, capped at 1000 chars.
    let (ok, out, err) = ctx.run_with_input_and_env(
        &["media", "add", "693134", "--kind", "movie", "--json"],
        None,
        &[("GENRL_TMDB_BASE_URL", &mock.uri())],
    );
    assert!(ok, "media add failed: {out} {err}");
    let add_res: serde_json::Value = serde_json::from_str(&out).expect("valid json output");
    let media_id = add_res["media_id"].as_i64().unwrap();

    // media list returns the stored poster_path and the capped overview.
    let (ok, out, _) = ctx.run(&["media", "list", "--json"]);
    assert!(ok, "media list failed: {out}");
    let list: serde_json::Value = serde_json::from_str(&out).expect("valid json output");
    let movie = list
        .as_array()
        .unwrap()
        .iter()
        .find(|it| it["id"] == media_id)
        .expect("added movie present in list");
    assert_eq!(movie["poster_path"], "/dune.jpg");
    assert_eq!(movie["overview"].as_str().unwrap().chars().count(), 1000);

    // raw_json holds ONLY the overview JSON, and never the API key.
    let db_path = ctx.temp_dir.path().join("genrl.db");
    let conn = rusqlite::Connection::open(&db_path).unwrap();
    let raw: Option<String> = conn
        .query_row(
            "SELECT raw_json FROM media WHERE id = ?1",
            [media_id],
            |r| r.get(0),
        )
        .unwrap();
    let raw = raw.expect("media add must store raw_json");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("raw_json is valid JSON");
    assert_eq!(
        parsed.as_object().unwrap().len(),
        1,
        "raw_json must hold only the overview"
    );
    assert!(parsed.get("overview").is_some());
    assert!(
        !raw.contains(KNOWN_FAKE_SECRET),
        "API key must never be stored in raw_json"
    );

    // Old row (added before overview existed): overview is null, poster untouched.
    conn.execute(
        "INSERT INTO media (kind, title, poster_path, created_at) VALUES ('movie', 'Legacy', '/legacy.jpg', 0)",
        [],
    )
    .unwrap();
    // Rows whose poster_path is not the strict /name.ext shape: poster_path is null.
    conn.execute(
        "INSERT INTO media (kind, title, poster_path, created_at) VALUES ('movie', 'Remote', 'https://evil.example/x.jpg', 0)",
        [],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO media (kind, title, poster_path, created_at) VALUES ('movie', 'Traversal', '/../secret.jpg', 0)",
        [],
    )
    .unwrap();
    drop(conn);

    let (ok, out, _) = ctx.run(&["media", "list", "--kind", "movie", "--json"]);
    assert!(ok, "media list failed: {out}");
    let list: serde_json::Value = serde_json::from_str(&out).expect("valid json output");
    let arr = list.as_array().unwrap();

    let legacy = arr.iter().find(|it| it["title"] == "Legacy").unwrap();
    assert_eq!(legacy["overview"], serde_json::Value::Null);
    assert_eq!(legacy["poster_path"], "/legacy.jpg");

    let remote = arr.iter().find(|it| it["title"] == "Remote").unwrap();
    assert_eq!(remote["poster_path"], serde_json::Value::Null);
    assert_eq!(remote["overview"], serde_json::Value::Null);

    let traversal = arr.iter().find(|it| it["title"] == "Traversal").unwrap();
    assert_eq!(traversal["poster_path"], serde_json::Value::Null);

    // The fake secret never appears in command output or the SQLite dump.
    let (ok, out, err) = ctx.run(&["media", "list", "--json"]);
    assert!(ok);
    assert!(
        !out.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in media list stdout!"
    );
    assert!(
        !err.contains(KNOWN_FAKE_SECRET),
        "Secret leaked in media list stderr!"
    );
    assert!(
        !ctx.dump_db().contains(KNOWN_FAKE_SECRET),
        "Secret leaked in SQLite database!"
    );
}
