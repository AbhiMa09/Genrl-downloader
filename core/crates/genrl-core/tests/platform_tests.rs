use std::fs;
use tempfile::tempdir;

use genrl_core::platform::{
    are_same_file, are_windows_file_indices_equal, are_windows_volumes_equal, default_download_dir,
    default_library_dir, extract_windows_volume, is_reserved_windows_name, sanitize_filename,
    set_file_permissions, transfer_file_with_fallback, MAX_FILENAME_LENGTH,
};
use genrl_core::settings::MoveMode;

#[test]
fn test_scratch_guard_panics_on_default_data_dir() {
    std::env::set_var("GENRL_REQUIRE_SCRATCH", "1");
    let result = std::panic::catch_unwind(|| {
        genrl_core::default_data_dir();
    });
    assert!(
        result.is_err(),
        "default_data_dir must panic when GENRL_REQUIRE_SCRATCH=1 is set"
    );
}

#[test]
fn test_windows_filename_sanitizer_reserved_names() {
    // Exact reserved DOS names
    assert!(is_reserved_windows_name("CON"));
    assert!(is_reserved_windows_name("prn"));
    assert!(is_reserved_windows_name("Aux"));
    assert!(is_reserved_windows_name("NUL"));
    assert!(is_reserved_windows_name("COM1"));
    assert!(is_reserved_windows_name("com9"));
    assert!(is_reserved_windows_name("LPT1"));
    assert!(is_reserved_windows_name("lpt9"));

    // Reserved names with extension
    assert!(is_reserved_windows_name("CON.mkv"));
    assert!(is_reserved_windows_name("nul.txt"));
    assert!(is_reserved_windows_name("aux.tar.gz"));

    // Non-reserved names that contain reserved stems as substrings
    assert!(!is_reserved_windows_name("CONFIG"));
    assert!(!is_reserved_windows_name("Connect"));
    assert!(!is_reserved_windows_name("Auxiliary"));

    // Sanitization prefixing
    assert_eq!(sanitize_filename("CON"), "_CON");
    assert_eq!(sanitize_filename("nul.mkv"), "_nul.mkv");
    assert_eq!(sanitize_filename("AUX.mp4"), "_AUX.mp4");
    assert_eq!(sanitize_filename("com1"), "_com1");
}

#[test]
fn test_windows_filename_sanitizer_illegal_chars() {
    // Characters: < > : " / \ | ? *
    let input = "Show: The <Final> \"Season\" / Part | Two? *Yes*";
    let sanitized = sanitize_filename(input);
    assert!(!sanitized.contains(':'));
    assert!(!sanitized.contains('<'));
    assert!(!sanitized.contains('>'));
    assert!(!sanitized.contains('"'));
    assert!(!sanitized.contains('/'));
    assert!(!sanitized.contains('\\'));
    assert!(!sanitized.contains('|'));
    assert!(!sanitized.contains('?'));
    assert!(!sanitized.contains('*'));
    assert_eq!(sanitized, "Show- The -Final- -Season- - Part - Two- -Yes");
}

#[test]
fn test_windows_filename_sanitizer_trailing_dots_and_spaces() {
    assert_eq!(sanitize_filename("Show Title. . "), "Show Title");
    assert_eq!(sanitize_filename("Episode 01..."), "Episode 01");
    assert_eq!(
        sanitize_filename("  Leading and Trailing   "),
        "Leading and Trailing"
    );
}

#[test]
fn test_windows_filename_sanitizer_length_cap() {
    let long_name = "A".repeat(300);
    let sanitized = sanitize_filename(&long_name);
    assert_eq!(sanitized.chars().count(), MAX_FILENAME_LENGTH);
    assert_eq!(sanitized, "A".repeat(MAX_FILENAME_LENGTH));
}

#[test]
fn test_windows_filename_sanitizer_empty_fallback() {
    assert_eq!(sanitize_filename(""), "Show");
    assert_eq!(sanitize_filename("   "), "Show");
    assert_eq!(sanitize_filename("..."), "Show");
    assert_eq!(sanitize_filename("---"), "Show");
    assert_eq!(sanitize_filename(" . - . "), "Show");
}

#[test]
fn test_windows_volume_comparison() {
    assert_eq!(
        extract_windows_volume(r"C:\Users\test"),
        Some("C:".to_string())
    );
    assert_eq!(
        extract_windows_volume(r"c:\another\path"),
        Some("C:".to_string())
    );
    assert_eq!(extract_windows_volume(r"D:\media"), Some("D:".to_string()));
    assert_eq!(
        extract_windows_volume(r"\\nas\anime\shows"),
        Some(r"\\nas\anime".to_string())
    );

    // Same drive letters (case insensitive)
    assert!(are_windows_volumes_equal(
        r"C:\Anime\Show",
        r"c:\Downloads\File"
    ));
    assert!(are_windows_volumes_equal(r"D:\a\b", r"d:\x\y"));

    // Cross-volume drives
    assert!(!are_windows_volumes_equal(
        r"C:\Anime\Show",
        r"D:\Downloads\File"
    ));
    assert!(!are_windows_volumes_equal(r"C:\Anime", r"E:\Anime"));

    // Network shares
    assert!(are_windows_volumes_equal(
        r"\\nas\anime\a",
        r"\\NAS\ANIME\b"
    ));
    assert!(!are_windows_volumes_equal(
        r"\\nas\anime\a",
        r"\\nas\movies\b"
    ));
}

#[test]
fn test_platform_chmod_no_op_or_permission() {
    let dir = tempdir().expect("temp dir");
    let file = dir.path().join("chmod_test.txt");
    fs::write(&file, b"content").expect("write test file");

    let res = set_file_permissions(&file, 0o644);
    assert!(res.is_ok(), "set_file_permissions must succeed");
}

#[test]
fn test_platform_transfer_file_with_fallback() {
    let dir = tempdir().expect("temp dir");
    let src = dir.path().join("source.mkv");
    let target_copy = dir.path().join("copy_target.mkv");
    let target_move = dir.path().join("move_target.mkv");
    let target_link = dir.path().join("link_target.mkv");

    fs::write(&src, b"video content payload").expect("write source");

    // Copy mode
    transfer_file_with_fallback(&src, &target_copy, MoveMode::Copy).expect("copy transfer");
    assert!(target_copy.exists());
    assert_eq!(fs::read(&target_copy).unwrap(), b"video content payload");

    // Hardlink mode (within same temp dir)
    transfer_file_with_fallback(&src, &target_link, MoveMode::Hardlink).expect("hardlink transfer");
    assert!(target_link.exists());
    assert_eq!(fs::read(&target_link).unwrap(), b"video content payload");

    // Move mode
    transfer_file_with_fallback(&src, &target_move, MoveMode::Move).expect("move transfer");
    assert!(target_move.exists());
    assert!(!src.exists(), "source must be unlinked after move");
}

#[test]
fn test_platform_default_download_and_library_dirs() {
    let dl = default_download_dir();
    let lib = default_library_dir();

    assert!(dl.to_string_lossy().ends_with("Genrl"));
    assert!(lib.to_string_lossy().ends_with("Anime"));
}

#[test]
fn test_are_windows_file_indices_equal() {
    assert!(are_windows_file_indices_equal(
        Some(12345),
        Some(67890),
        Some(12345),
        Some(67890),
    ));
    // Different volume serial
    assert!(!are_windows_file_indices_equal(
        Some(12345),
        Some(67890),
        Some(99999),
        Some(67890),
    ));
    // Different file index
    assert!(!are_windows_file_indices_equal(
        Some(12345),
        Some(67890),
        Some(12345),
        Some(11111),
    ));
    // Missing volume serial or file index
    assert!(!are_windows_file_indices_equal(
        None,
        Some(67890),
        Some(12345),
        Some(67890),
    ));
    assert!(!are_windows_file_indices_equal(
        Some(12345),
        None,
        Some(12345),
        Some(67890),
    ));
}

#[test]
fn test_are_same_file_detection() {
    let dir = tempdir().unwrap();
    let file1 = dir.path().join("file1.bin");
    let file2 = dir.path().join("file2.bin");
    let file3 = dir.path().join("file3.bin");

    fs::write(&file1, b"hello").unwrap();
    fs::hard_link(&file1, &file2).unwrap();
    fs::write(&file3, b"hello").unwrap();

    assert!(
        are_same_file(&file1, &file2),
        "hardlinked files must be detected as same file"
    );
    assert!(
        !are_same_file(&file1, &file3),
        "independent files must not be detected as same file"
    );
}

use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt;

struct LogCapture(Arc<Mutex<Vec<String>>>);
impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for LogCapture {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogVisitor(String::new());
        event.record(&mut visitor);
        self.0.lock().unwrap().push(visitor.0);
    }
}

struct LogVisitor(String);
impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, _field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!("{value:?} "));
    }
    fn record_str(&mut self, _field: &tracing::field::Field, value: &str) {
        self.0.push_str(value);
    }
}

#[test]
fn test_hardlink_fallback_records_warning() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("source.mkv");
    let target = dir.path().join("target.mkv");

    fs::write(&src, b"new content").expect("write source");
    // Create target so hard_link fails with AlreadyExists
    fs::write(&target, b"old content").expect("write target");

    let logs = Arc::new(Mutex::new(Vec::new()));
    let subscriber = tracing_subscriber::registry().with(LogCapture(Arc::clone(&logs)));

    tracing::subscriber::with_default(subscriber, || {
        transfer_file_with_fallback(&src, &target, MoveMode::Hardlink).expect("transfer");
    });

    let recorded = logs.lock().unwrap().clone();
    assert_eq!(fs::read(&target).unwrap(), b"new content");
    assert!(
        recorded
            .iter()
            .any(|msg| msg.contains("hardlink failed") && msg.contains("falling back to copy")),
        "fallback must record a warning naming hardlink failure: {recorded:?}"
    );
}
