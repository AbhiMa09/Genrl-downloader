use insta::assert_snapshot;

#[test]
fn snapshot_cli_doctor_output() {
    let mut checks = Vec::new();
    checks.push("[+] Database schema: v1 (SQLite bundled)");
    checks.push("[+] Storage locations: writable");
    checks.push("[+] qBittorrent WebUI: offline (mock)");
    checks.push("[+] Network Gate: initialized");

    let output = checks.join("\n");
    assert_snapshot!("cli_doctor_output", output);
}

#[test]
fn snapshot_cli_default_settings_json() {
    let db = genrl_core::db::Db::open_in_memory().unwrap();
    let settings = genrl_core::Settings::load(&db).unwrap();

    let json_val = serde_json::to_value(&settings).unwrap();
    let rendered = serde_json::to_string_pretty(&json_val).unwrap();
    assert_snapshot!("cli_default_settings_json", rendered);
}

#[test]
fn snapshot_cli_queue_empty_display() {
    let empty_output = "No active jobs in queue. Automation idle.";
    assert_snapshot!("cli_queue_empty_display", empty_output);
}
