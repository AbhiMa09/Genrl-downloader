use std::fs;
use tempfile::tempdir;

use genrl_core::platform::{
    are_windows_volumes_equal, default_download_dir, default_library_dir, extract_windows_volume,
    is_reserved_windows_name, sanitize_filename, set_file_permissions, transfer_file_with_fallback,
    MAX_FILENAME_LENGTH,
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
