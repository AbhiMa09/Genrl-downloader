//! Locations integration and unit tests.
//!
//! Tests:
//! 1. Path safety validation (reject traversal, empty).
//! 2. Directory check (missing, created, writable, read-only permissions).
//! 3. Filesystem check (same filesystem detection).
//! 4. Settings get/set locations and event logging.
//! 5. Per-show folder override wins and never silently falls back.
//! 6. Explicit confirmed library migration moves files and updates show folder paths.

use std::fs;
use std::path::Path;

use genrl_core::db::Db;
use genrl_core::events::EventLog;
use genrl_core::library::resolve_show_folder;
use genrl_core::locations::{
    are_on_same_filesystem, check_directory, default_download_dir, default_library_dir,
    get_locations, get_show_folder, move_existing_library, set_location, validate_path_safety,
};

#[test]
fn path_safety_rejects_traversal_and_empty() {
    assert!(validate_path_safety(Path::new(""), None).is_err());
    assert!(validate_path_safety(Path::new("   "), None).is_err());
    assert!(validate_path_safety(Path::new("/home/user/../etc/passwd"), None).is_err());
    assert!(validate_path_safety(Path::new("/media/anime/../../root"), None).is_err());

    // Valid absolute path succeeds
    let valid = validate_path_safety(Path::new("/var/media/anime"), None);
    assert!(valid.is_ok());
    assert_eq!(valid.unwrap(), Path::new("/var/media/anime"));
}

#[test]
fn directory_validation_missing_and_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("sub").join("anime_library");

    // When create_if_missing is false, missing dir reports exists=false
    let check = check_directory(&target.to_string_lossy(), false);
    assert!(!check.exists);
    assert!(!check.path.is_empty());

    // When create_if_missing is true, directory is created
    let check_created = check_directory(&target.to_string_lossy(), true);
    assert!(check_created.exists);
    assert!(check_created.writable);
    assert!(check_created.is_valid);
    assert!(check_created.free_space_bytes.is_some());
    assert!(check_created.free_space_gb.is_some());
    assert!(target.exists());
}

#[test]
#[cfg(unix)]
fn directory_validation_read_only_folder() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let ro_dir = tmp.path().join("read_only_dir");
    fs::create_dir_all(&ro_dir).unwrap();

    // Set permissions to read-only (0o555 for dir so it can be traversed/read but not written)
    fs::set_permissions(&ro_dir, fs::Permissions::from_mode(0o555)).unwrap();

    let check = check_directory(&ro_dir.to_string_lossy(), false);
    assert!(check.exists);
    assert!(!check.writable, "read-only folder must not be writable");
    assert!(
        !check.is_valid,
        "read-only folder is invalid for library/downloads"
    );
    assert!(check.error.is_some());

    // Restore permissions so tempdir cleanup succeeds
    let _ = fs::set_permissions(&ro_dir, fs::Permissions::from_mode(0o755));
}

#[test]
fn same_filesystem_detection() {
    let tmp = tempfile::tempdir().unwrap();
    let dir_a = tmp.path().join("downloads");
    let dir_b = tmp.path().join("library");
    fs::create_dir_all(&dir_a).unwrap();
    fs::create_dir_all(&dir_b).unwrap();

    assert!(
        are_on_same_filesystem(&dir_a, &dir_b),
        "two folders in the same tempdir must be on the same filesystem"
    );
}

#[test]
fn get_and_set_locations_with_event_log() {
    let db = Db::open_in_memory().unwrap();
    let log = EventLog::new(db.clone());

    // Default locations
    let report = get_locations(&db).unwrap();
    assert_eq!(
        report.download_dir.path,
        default_download_dir().display().to_string()
    );
    assert_eq!(
        report.library_dir.path,
        default_library_dir().display().to_string()
    );

    // Update download folder
    let tmp = tempfile::tempdir().unwrap();
    let new_dl = tmp.path().join("custom_dl");
    let check = set_location(&db, "download_dir", &new_dl.to_string_lossy(), None, &log).unwrap();
    assert!(check.exists);
    assert!(check.writable);

    // Verify it persisted
    let updated = get_locations(&db).unwrap();
    assert_eq!(updated.download_dir.path, new_dl.display().to_string());

    // Verify event log
    let events = log.recent(10).unwrap();
    assert!(events
        .iter()
        .any(|e| e.message.contains("Updated download folder")));
}

#[test]
fn per_show_folder_override_wins_and_never_falls_back() {
    let db = Db::open_in_memory().unwrap();
    let log = EventLog::new(db.clone());
    let tmp = tempfile::tempdir().unwrap();
    let lib_dir = tmp.path().join("default_library");
    fs::create_dir_all(&lib_dir).unwrap();

    // Create a show
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, start_year, season, auto)
             VALUES (101, 'Frieren', 'tv', 2023, 1, 1)",
            [],
        )?;
        Ok(())
    })
    .unwrap();

    // Without override, resolve_show_folder uses library_dir/<Show>
    let default_path = resolve_show_folder(&db, &lib_dir, 101, "Frieren", Some(2023)).unwrap();
    assert_eq!(default_path, lib_dir.join("Frieren (2023)"));
    assert!(default_path.exists());

    // Now set per-show folder override
    let override_dir = tmp.path().join("Special_Anime").join("Sousou_no_Frieren");
    let check = set_location(
        &db,
        "folder_path",
        &override_dir.to_string_lossy(),
        Some(101),
        &log,
    )
    .unwrap();
    assert!(check.exists);

    // Verify get_show_folder
    let show_info = get_show_folder(&db, 101).unwrap();
    assert_eq!(
        show_info.folder_path,
        Some(override_dir.display().to_string())
    );

    // With override, resolve_show_folder returns the override path strictly
    let resolved = resolve_show_folder(&db, &lib_dir, 101, "Frieren", Some(2023)).unwrap();
    assert_eq!(resolved, override_dir);

    // When override points to a path that cannot be created (e.g. inside an existing regular file),
    // it MUST return Err and NEVER silently fall back to lib_dir!
    let blocked_parent_file = tmp.path().join("not_a_dir.txt");
    fs::write(&blocked_parent_file, b"content").unwrap();
    let invalid_override = blocked_parent_file.join("cannot_be_created");

    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, folder_path) VALUES (102, 'Bocchi', 'tv', ?1)",
            rusqlite::params![invalid_override.display().to_string()],
        )?;
        Ok(())
    })
    .unwrap();

    let result = resolve_show_folder(&db, &lib_dir, 102, "Bocchi", Some(2022));
    assert!(
        result.is_err(),
        "Must error out rather than silently falling back to a different folder"
    );
}

#[test]
fn explicit_library_migration() {
    let db = Db::open_in_memory().unwrap();
    let log = EventLog::new(db.clone());
    let tmp = tempfile::tempdir().unwrap();

    let old_lib = tmp.path().join("old_library");
    let new_lib = tmp.path().join("new_library");

    // Populate old library with a show and episode
    let show_dir = old_lib.join("Frieren");
    fs::create_dir_all(&show_dir).unwrap();
    fs::write(
        show_dir.join("Frieren - S01E01.mkv"),
        b"video content episode 1",
    )
    .unwrap();
    fs::write(show_dir.join("synopsis.txt"), b"synopsis info").unwrap();

    // Register show in DB pointing to old_lib/Frieren
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, type, folder_path) VALUES (1, 'Frieren', 'tv', ?1)",
            rusqlite::params![show_dir.display().to_string()],
        )?;
        Ok(())
    })
    .unwrap();

    // Perform confirmed migration
    let report = move_existing_library(&db, &old_lib, &new_lib, &log).unwrap();
    assert_eq!(report.shows_moved, 1);
    assert_eq!(report.files_moved, 2);
    assert!(report.bytes_moved > 0);
    assert!(report.errors.is_empty());

    // Verify files exist in new location
    let new_show_dir = new_lib.join("Frieren");
    assert!(new_show_dir.join("Frieren - S01E01.mkv").exists());
    assert!(new_show_dir.join("synopsis.txt").exists());

    // Verify show folder_path was updated in database
    let updated_path: String = db
        .with(|c| {
            Ok(
                c.query_row("SELECT folder_path FROM shows WHERE mal_id = 1", [], |r| {
                    r.get(0)
                })?,
            )
        })
        .unwrap();
    assert_eq!(updated_path, new_show_dir.display().to_string());
}
