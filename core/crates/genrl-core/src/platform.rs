//! Platform abstraction layer for cross-platform compatibility (Linux / Windows / macOS).
//!
//! Provides OS-specific directory resolution (%APPDATA%\Genrl Downloader on Windows,
//! XDG on Linux), permission handling, volume/filesystem checks, hardlink fallback,
//! and Windows-safe filename sanitization (handling reserved device names, forbidden
//! characters, trailing dots/spaces, and path length bounds).

use std::fs;
use std::path::{Path, PathBuf};

/// Where the app keeps its data when `--data-dir` is not specified.
///
/// On Windows: `%APPDATA%\Genrl Downloader`
/// On Linux/Unix: `$XDG_DATA_HOME/genrl-downloader` or `~/.local/share/genrl-downloader`
pub fn default_data_dir() -> PathBuf {
    if std::env::var("GENRL_REQUIRE_SCRATCH").is_ok() {
        panic!("Refusing to resolve default_data_dir(): GENRL_REQUIRE_SCRATCH=1 is set. All data paths must come from the single data-dir the process was started with.");
    }

    #[cfg(windows)]
    {
        if let Some(appdata) = dirs::data_dir() {
            return appdata.join("Genrl Downloader");
        }
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("Genrl Downloader");
            }
        }
        PathBuf::from(r"C:\Genrl Downloader")
    }

    #[cfg(not(windows))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return PathBuf::from(xdg).join("genrl-downloader");
            }
        }
        if let Some(data) = dirs::data_dir() {
            return data.join("genrl-downloader");
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("genrl-downloader")
    }
}

/// Path of the SQLite database inside the default data directory.
pub fn default_db_path() -> PathBuf {
    default_data_dir().join("genrl.db")
}

/// Default download folder (`Downloads\Genrl` in user's home/download folder).
pub fn default_download_dir() -> PathBuf {
    if let Some(dl) = dirs::download_dir() {
        return dl.join("Genrl");
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("Downloads").join("Genrl")
}

/// Default library folder (`Videos\Anime` in user's home/video folder).
pub fn default_library_dir() -> PathBuf {
    if let Some(vids) = dirs::video_dir() {
        return vids.join("Anime");
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join("Videos").join("Anime")
}

/// Set file permissions on Unix; no-op on Windows.
pub fn set_file_permissions(path: &Path, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
    }
    Ok(())
}

/// Check whether two paths reside on the same filesystem / volume.
///
/// On Windows: validates identical drive letters or UNC volume roots.
/// On Unix: compares filesystem device IDs (`MetadataExt::dev`).
pub fn are_on_same_volume(path_a: &Path, path_b: &Path) -> bool {
    let a_str = path_a.to_string_lossy();
    let b_str = path_b.to_string_lossy();

    // Check Windows volume prefixes (e.g. C:\ vs D:\ or \\server\share)
    if is_windows_style_path(&a_str) || is_windows_style_path(&b_str) {
        return are_windows_volumes_equal(&a_str, &b_str);
    }

    #[cfg(unix)]
    {
        let dev_a = get_dev_for_path(path_a);
        let dev_b = get_dev_for_path(path_b);
        if let (Some(a), Some(b)) = (dev_a, dev_b) {
            return a == b;
        }
    }

    false
}

#[cfg(unix)]
fn get_dev_for_path(p: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    let mut curr = p.to_path_buf();
    loop {
        if let Ok(meta) = fs::metadata(&curr) {
            return Some(meta.dev());
        }
        if !curr.pop() {
            break;
        }
    }
    None
}

/// Detect if string has Windows drive prefix or UNC prefix
fn is_windows_style_path(p: &str) -> bool {
    let bytes = p.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return true;
    }
    p.starts_with(r"\\") || p.starts_with("//")
}

/// Compare Windows volume roots (case-insensitively)
pub fn are_windows_volumes_equal(p1: &str, p2: &str) -> bool {
    let vol1 = extract_windows_volume(p1);
    let vol2 = extract_windows_volume(p2);
    match (vol1, vol2) {
        (Some(v1), Some(v2)) => v1.eq_ignore_ascii_case(&v2),
        _ => false,
    }
}

/// Compare Windows volume serial and file index identifiers.
pub fn are_windows_file_indices_equal(
    v1: Option<u32>,
    i1: Option<u64>,
    v2: Option<u32>,
    i2: Option<u64>,
) -> bool {
    match (v1, v2, i1, i2) {
        (Some(vol1), Some(vol2), Some(idx1), Some(idx2)) => vol1 == vol2 && idx1 == idx2,
        _ => false,
    }
}

/// Check if two files refer to the exact same file on disk (same device & inode on Unix,
/// or same volume serial number & file index on Windows).
pub fn are_same_file(m1: &fs::Metadata, m2: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        m1.dev() == m2.dev() && m1.ino() == m2.ino()
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        are_windows_file_indices_equal(
            m1.volume_serial_number(),
            m1.file_index(),
            m2.volume_serial_number(),
            m2.file_index(),
        )
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (m1, m2);
        false
    }
}

/// Extract drive letter or UNC prefix from a Windows path string.
pub fn extract_windows_volume(p: &str) -> Option<String> {
    let trimmed = p.trim();
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        return Some(format!("{}:", (bytes[0] as char).to_ascii_uppercase()));
    }
    if trimmed.starts_with(r"\\") || trimmed.starts_with("//") {
        let norm = trimmed.replace('/', "\\");
        let parts: Vec<&str> = norm.split('\\').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            return Some(format!(
                r"\\{}\{}",
                parts[0].to_ascii_lowercase(),
                parts[1].to_ascii_lowercase()
            ));
        }
    }
    None
}

/// Maximum safe component length across filesystems (NTFS & ext4).
pub const MAX_FILENAME_LENGTH: usize = 120;

/// Windows reserved DOS device names.
const RESERVED_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Check whether a name (or its stem) matches a Windows reserved device name.
pub fn is_reserved_windows_name(name: &str) -> bool {
    let stem = match name.find('.') {
        Some(pos) => &name[..pos],
        None => name,
    };
    let trimmed = stem.trim();
    RESERVED_DEVICE_NAMES
        .iter()
        .any(|&res| res.eq_ignore_ascii_case(trimmed))
}

/// Sanitize filename for all platforms, strictly enforcing Windows/NTFS constraints.
///
/// Disallows:
/// - Forbidden characters: `< > : " / \ | ? * \0`
/// - ASCII control characters (0..31) and delete/C1 (127..159)
/// - Unicode bidi overrides (U+200E, U+200F, U+202A..U+202E, U+2066..U+2069)
/// - Reserved Windows device names (CON, NUL, AUX, PRN, COM1-9, LPT1-9)
/// - Trailing dots and spaces
/// - Total component length exceeding MAX_FILENAME_LENGTH
pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        if matches!(
            c,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'
        ) {
            out.push('-');
            continue;
        }
        if c < ' ' || ('\x7f'..='\u{9F}').contains(&c) {
            continue;
        }
        if matches!(
            c,
            '\u{200E}'
                | '\u{200F}'
                | '\u{202A}'
                ..='\u{202E}'
                | '\u{2066}'
                ..='\u{2069}'
        ) {
            continue;
        }
        out.push(c);
    }

    // Windows disallows trailing dots and spaces
    let trimmed = out
        .trim()
        .trim_matches(|c| c == '.' || c == '-' || c == ' ')
        .trim();
    if trimmed.is_empty() {
        return "Show".to_string();
    }

    // If matches a reserved Windows name (e.g. CON, NUL, aux.mkv), prefix with '_'
    let safe_stem = if is_reserved_windows_name(trimmed) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    };

    // Ensure within MAX_FILENAME_LENGTH characters safely
    let chars: Vec<char> = safe_stem.chars().collect();
    let result: String = if chars.len() > MAX_FILENAME_LENGTH {
        chars[..MAX_FILENAME_LENGTH].iter().collect()
    } else {
        safe_stem
    };

    // Final trim of trailing dots/spaces in case truncation produced one
    let final_clean = result.trim_end_matches(['.', ' ']).trim();
    if final_clean.is_empty() {
        "Show".to_string()
    } else {
        final_clean.to_string()
    }
}

/// Transfer a file to the target path with fallback for cross-volume hardlinks.
pub fn transfer_file_with_fallback(
    source: &Path,
    target: &Path,
    mode: crate::settings::MoveMode,
) -> std::io::Result<()> {
    if source == target {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    match mode {
        crate::settings::MoveMode::Hardlink => {
            // Check same volume first if known
            let can_hardlink = are_on_same_volume(source, target);
            if can_hardlink {
                if fs::hard_link(source, target).is_err() {
                    fs::copy(source, target)?;
                }
            } else {
                // Different volume or unknown: try hard_link, fall back to copy
                if fs::hard_link(source, target).is_err() {
                    fs::copy(source, target)?;
                }
            }
        }
        crate::settings::MoveMode::Move => {
            if fs::rename(source, target).is_err() {
                fs::copy(source, target)?;
                let _ = fs::remove_file(source);
            }
        }
        crate::settings::MoveMode::Copy => {
            fs::copy(source, target)?;
        }
    }

    let _ = set_file_permissions(target, 0o644);
    Ok(())
}
