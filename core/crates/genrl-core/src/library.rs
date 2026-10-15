//! The library step: file placement, metadata, and folder decoration (Phase 6).
//!
//! When a torrent finishes, the finalize step files it in a predictable layout:
//!
//! ```text
//! Anime/
//!   Kimi no Na wa (2016)/
//!     synopsis.txt
//!     folder.png
//!     .directory
//!     Kimi no Na wa (2016).mkv
//!   Show Title (2024)/
//!     synopsis.txt
//!     folder.png
//!     .directory
//!     Show Title - S01E01.mkv
//!     Show Title - S01E02.mkv
//! ```
//!
//! The step follows the six design rules:
//!
//! 1. **Verify the file:** video extension allowlist, file existence and size.
//! 2. **Resolve show folder:** matched by `.mal_id` marker file first so
//!    renaming folders never loses show identity; sanitized names truncate at
//!    120 characters and strip control/bidi characters.
//! 3. **Write synopsis.txt:** rich metadata from Jikan, sanitized against
//!    injection; writes a placeholder if Jikan is offline/down instead of
//!    failing the download.
//! 4. **Download poster & set folder icon:** square `folder.png` via `magick`
//!    or pure-Rust fallback; sets `.directory` (KDE) or `gio` (GNOME).
//! 5. **Move or hardlink:** according to settings, with cross-device copy fallback
//!    and `chmod 644`.
//! 6. **Handle collisions:** asks in the Inbox (`FileExists`) rather than
//!    overwriting silently, honoring `Keep`, `Replace`, or `KeepBoth`.
//! 7. **Rescan folders:** adopts existing directories and episodes without duplication.

use std::collections::HashSet;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::EventLog;
use crate::http::{Gate, GetOptions, Service};
use crate::models::ShowType;
use crate::questions::{Answer, Choice, Payload, QuestionStore};
use crate::settings::{IconMethod, IconStyle, MoveMode, Settings};
use crate::worker::{FinalizeContext, FinalizeOutcome, FinalizeSuccess, Finalizer};

/// Allowed extensions for files placed in the library (Threat model §1).
pub const ALLOWED_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "webm", "m4v", "srt", "ass", "ssa", "sub", "png", "jpg", "jpeg",
];

/// Video extensions recognized as episode payloads.
pub const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "webm", "m4v"];

/// Marker file inside every show folder holding its MAL id.
pub const MARKER_FILE: &str = ".mal_id";

/// Synopsis file written once per show.
pub const SYNOPSIS_FILE: &str = "synopsis.txt";

/// Square folder icon file.
pub const FOLDER_ICON_FILE: &str = "folder.png";

/// KDE Plasma desktop folder configuration.
pub const DIRECTORY_FILE: &str = ".directory";

/// Maximum length of a sanitized folder name component.
pub const MAX_NAME_LENGTH: usize = 120;

/// Report from a library rescan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RescanReport {
    pub shows_adopted: usize,
    pub episodes_found: usize,
}

// ---------------------------------------------------------------------------
// Jikan API Wire Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct JikanResponse {
    data: JikanAnimeData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAnimeData {
    pub mal_id: i64,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub title_english: Option<String>,
    #[serde(default)]
    pub title_japanese: Option<String>,
    #[serde(rename = "type", default)]
    pub show_type: Option<String>,
    #[serde(default)]
    pub episodes: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub aired: Option<JikanAired>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(default)]
    pub genres: Vec<JikanGenre>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub images: Option<JikanImages>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAired {
    #[serde(default)]
    pub string: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanGenre {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanImages {
    #[serde(default)]
    pub jpg: Option<JikanImageUrls>,
    #[serde(default)]
    pub webp: Option<JikanImageUrls>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanImageUrls {
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub large_image_url: Option<String>,
}

// ---------------------------------------------------------------------------
// Sanitization and Formatting Helpers
// ---------------------------------------------------------------------------

/// Strip path separators, control characters, bidi overrides and leading/trailing dots.
pub fn sanitize_component(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        // Disallow path separators and unsafe filename characters
        if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0') {
            out.push('-');
            continue;
        }
        // Disallow control characters
        if c < ' ' || (c >= '\x7f' && c <= '\u{9F}') {
            continue;
        }
        // Disallow Unicode bidi overrides (Threat model §2)
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

    let trimmed = out.trim().trim_matches(|c| c == '.' || c == '-').trim();
    if trimmed.is_empty() {
        return "Show".to_string();
    }

    // Truncate to MAX_NAME_LENGTH characters safely
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() > MAX_NAME_LENGTH {
        chars[..MAX_NAME_LENGTH].iter().collect()
    } else {
        chars.iter().collect()
    }
}

/// Strip newlines and control characters from single-line headers to prevent injection.
pub fn sanitize_single_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '\r' || c == '\n' {
            out.push(' ');
        } else if c >= ' ' && c != '\x7f' {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Is this an allowed video file extension?
pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Format the synopsis.txt contents according to the design specification.
pub fn format_synopsis(
    title: &str,
    english_title: Option<&str>,
    japanese_title: Option<&str>,
    show_type: &str,
    episode_count: Option<i64>,
    airing_status: Option<&str>,
    airing_dates: Option<&str>,
    mal_score: Option<f64>,
    genres: &[String],
    synopsis_text: Option<&str>,
    mal_id: i64,
    date_written: &str,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("Title: {}\n", sanitize_single_line(title)));
    if let Some(en) = english_title {
        if !en.trim().is_empty() {
            out.push_str(&format!("English title: {}\n", sanitize_single_line(en)));
        }
    }
    if let Some(ja) = japanese_title {
        if !ja.trim().is_empty() {
            out.push_str(&format!("Japanese title: {}\n", sanitize_single_line(ja)));
        }
    }
    out.push_str(&format!("Type: {}\n", sanitize_single_line(show_type)));
    if let Some(ep) = episode_count {
        out.push_str(&format!("Episodes: {}\n", ep));
    } else {
        out.push_str("Episodes: Unknown\n");
    }
    if let Some(st) = airing_status {
        out.push_str(&format!("Status: {}\n", sanitize_single_line(st)));
    }
    if let Some(dates) = airing_dates {
        out.push_str(&format!("Aired: {}\n", sanitize_single_line(dates)));
    }
    if let Some(score) = mal_score {
        out.push_str(&format!("MAL score: {:.2}\n", score));
    } else {
        out.push_str("MAL score: N/A\n");
    }
    if !genres.is_empty() {
        let clean_genres: Vec<String> = genres.iter().map(|g| sanitize_single_line(g)).collect();
        out.push_str(&format!("Genres: {}\n", clean_genres.join(", ")));
    }
    out.push_str(&format!("MAL URL: https://myanimelist.net/anime/{mal_id}\n"));
    out.push_str(&format!("Date written: {}\n\n", sanitize_single_line(date_written)));

    out.push_str("Synopsis:\n");
    match synopsis_text {
        Some(text) if !text.trim().is_empty() => {
            out.push_str(text.trim());
            out.push('\n');
        }
        _ => {
            out.push_str("[Synopsis unavailable; will refresh when Jikan is reachable]\n");
        }
    }

    out
}

/// Check if a synopsis text is a placeholder.
pub fn is_placeholder_synopsis(content: &str) -> bool {
    let t = content.trim();
    t.is_empty()
        || t.starts_with("[Synopsis pending")
        || t.starts_with("[Synopsis unavailable")
        || t.starts_with("[Synopsis loading")
}

/// Format the NFO metadata file according to Kodi / Jellyfin / Emby XML spec (plot, rating, year).
pub fn format_nfo(
    tag: &str,
    title: &str,
    original_title: Option<&str>,
    plot: &str,
    rating: Option<f64>,
    year: Option<i64>,
    mal_id: i64,
) -> String {
    let eng = original_title.unwrap_or(title);
    let rating_str = rating.map(|r| format!("{:.2}", r)).unwrap_or_default();
    let year_str = year.map(|y| y.to_string()).unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<{tag}>\n  <title>{title}</title>\n  <originaltitle>{eng}</originaltitle>\n  <plot>{plot}</plot>\n  <rating>{rating_str}</rating>\n  <year>{year_str}</year>\n  <uniqueid type=\"mal\" default=\"true\">{mal_id}</uniqueid>\n</{tag}>\n"
    )
}

/// Format an episode's filename based on settings patterns.
pub fn format_episode_filename(
    pattern: &str,
    show_title: &str,
    season: i64,
    episode: f64,
    year: Option<i64>,
    ext: &str,
) -> String {
    let clean_title = sanitize_component(show_title);
    let year_str = year.map(|y| y.to_string()).unwrap_or_default();
    let ext_str = ext.trim_start_matches('.').to_ascii_lowercase();

    let ep_str = if episode.fract() == 0.0 {
        format!("{:.0}", episode)
    } else {
        format!("{}", episode)
    };

    let ep_pad = if episode.fract() == 0.0 {
        format!("{:02.0}", episode)
    } else {
        format!("{:04.1}", episode)
    };

    let mut result = pattern
        .replace("{title}", &clean_title)
        .replace("{year}", &year_str)
        .replace("{season:02}", &format!("{:02}", season))
        .replace("{season}", &format!("{}", season))
        .replace("{episode:02}", &ep_pad)
        .replace("{episode}", &ep_str)
        .replace("{ext}", &ext_str);

    let expected_suffix = format!(".{}", ext_str);
    if !result.ends_with(&expected_suffix) {
        result.push_str(&expected_suffix);
    }
    result
}

// ---------------------------------------------------------------------------
// Solid PNG Generator Fallback
// ---------------------------------------------------------------------------

/// Standard IEEE 802.3 CRC32 calculation.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Generate a valid 512x512 PNG with a solid background color (pure Rust fallback).
pub fn write_solid_png(path: &Path, width: u32, height: u32, r: u8, g: u8, b: u8) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = fs::File::create(path)?;
    // 8-byte PNG signature
    file.write_all(b"\x89PNG\r\n\x1a\n")?;

    // Helper to write a PNG chunk with CRC
    let write_chunk = |file: &mut fs::File, tag: &[u8; 4], data: &[u8]| -> std::io::Result<()> {
        let len = data.len() as u32;
        file.write_all(&len.to_be_bytes())?;
        file.write_all(tag)?;
        file.write_all(data)?;
        let mut crc_buf = Vec::with_capacity(4 + data.len());
        crc_buf.extend_from_slice(tag);
        crc_buf.extend_from_slice(data);
        let crc_val = crc32(&crc_buf);
        file.write_all(&crc_val.to_be_bytes())?;
        Ok(())
    };

    // IHDR chunk
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type: RGB
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    // Construct raw scanlines: filter byte 0 followed by width * 3 bytes
    let line_len = 1 + (width as usize) * 3;
    let mut raw = Vec::with_capacity(line_len * (height as usize));
    for _ in 0..height {
        raw.push(0); // filter: None
        for _ in 0..width {
            raw.push(r);
            raw.push(g);
            raw.push(b);
        }
    }

    // zlib wrapping for IDAT
    let mut zlib_data = Vec::new();
    zlib_data.push(0x78); // zlib header
    zlib_data.push(0x01); // compression level

    // Split raw into uncompressed DEFLATE blocks (max 65535 bytes per block)
    let chunks = raw.chunks(65535);
    let chunk_count = chunks.len();
    for (i, chunk) in chunks.enumerate() {
        let is_last = i == chunk_count - 1;
        zlib_data.push(if is_last { 0x01 } else { 0x00 });
        let len = chunk.len() as u16;
        let nlen = !len;
        zlib_data.extend_from_slice(&len.to_le_bytes());
        zlib_data.extend_from_slice(&nlen.to_le_bytes());
        zlib_data.extend_from_slice(chunk);
    }

    // Adler-32 checksum
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &byte in &raw {
        s1 = (s1 + byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    let adler = (s2 << 16) | s1;
    zlib_data.extend_from_slice(&adler.to_be_bytes());

    write_chunk(&mut file, b"IDAT", &zlib_data)?;
    write_chunk(&mut file, b"IEND", &[])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Desktop Folder Icons
// ---------------------------------------------------------------------------

/// Create the folder.png icon file (square 512x512).
pub fn create_folder_png(
    poster_src: &Path,
    folder_png_dst: &Path,
    _style: IconStyle,
) -> Result<()> {
    // Try ImageMagick (magick or convert) first if available
    let binaries = ["magick", "convert"];
    for bin in binaries {
        let status = Command::new(bin)
            .arg(poster_src)
            .args(["-resize", "512x512^", "-gravity", "center", "-extent", "512x512"])
            .arg(folder_png_dst)
            .status();
        if let Ok(st) = status {
            if st.success() && folder_png_dst.exists() {
                return Ok(());
            }
        }
    }

    // Fallback: If poster_src is already a valid file, try copying it directly
    if poster_src.exists() {
        if let Ok(_) = fs::copy(poster_src, folder_png_dst) {
            return Ok(());
        }
    }

    // Fallback: Generate a dark violet square PNG (64x64)
    write_solid_png(folder_png_dst, 64, 64, 30, 20, 50)
        .map_err(|e| Error::Io(e))?;
    Ok(())
}

/// Apply folder icon based on the desktop environment setting.
pub fn apply_folder_icon(
    method: IconMethod,
    show_dir: &Path,
    folder_png_path: &Path,
) -> Result<()> {
    match method {
        IconMethod::Off => Ok(()),
        IconMethod::Directory => write_directory_file(show_dir),
        IconMethod::Gio => set_gio_custom_icon(show_dir, folder_png_path),
        IconMethod::Auto => {
            let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_ascii_lowercase();
            if desktop.contains("gnome") {
                if set_gio_custom_icon(show_dir, folder_png_path).is_err() {
                    let _ = write_directory_file(show_dir);
                }
            } else {
                let _ = write_directory_file(show_dir);
            }
            Ok(())
        }
    }
}

fn write_directory_file(show_dir: &Path) -> Result<()> {
    let directory_file = show_dir.join(DIRECTORY_FILE);
    let contents = "[Desktop Entry]\nIcon=folder.png\n";
    fs::write(&directory_file, contents)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&directory_file, fs::Permissions::from_mode(0o644));
    }
    Ok(())
}

fn set_gio_custom_icon(show_dir: &Path, folder_png_path: &Path) -> Result<()> {
    let uri = format!("file://{}", folder_png_path.display());
    let status = Command::new("gio")
        .args(["set", "-t", "string"])
        .arg(show_dir)
        .arg("metadata::custom-icon")
        .arg(&uri)
        .status();
    match status {
        Ok(st) if st.success() => Ok(()),
        Ok(st) => Err(Error::Other(format!("gio set failed with status {st}"))),
        Err(e) => Err(Error::Io(e)),
    }
}

// ---------------------------------------------------------------------------
// File Transfer & Permissions
// ---------------------------------------------------------------------------

/// Move, copy, or hardlink a file into the library with chmod 644.
pub fn transfer_file(source: &Path, target: &Path, mode: MoveMode) -> Result<()> {
    if source == target {
        return Ok(());
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    match mode {
        MoveMode::Hardlink => {
            if let Err(_) = fs::hard_link(source, target) {
                // Cross-device link fallback: copy
                fs::copy(source, target)?;
            }
        }
        MoveMode::Move => {
            if let Err(_) = fs::rename(source, target) {
                // Cross-device rename fallback: copy then unlink
                fs::copy(source, target)?;
                let _ = fs::remove_file(source);
            }
        }
        MoveMode::Copy => {
            fs::copy(source, target)?;
        }
    }

    // Strip execute bits (chmod 644)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(target, fs::Permissions::from_mode(0o644));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Show Folder Resolution
// ---------------------------------------------------------------------------

/// Resolve or create the show folder in the library root.
pub fn resolve_show_folder(
    db: &Db,
    library_folder: &Path,
    mal_id: i64,
    title: &str,
    start_year: Option<i64>,
) -> Result<PathBuf> {
    // 1. Check database for existing folder_path
    let existing_path: Option<String> = db.with(|c| {
        Ok(c.query_row(
            "SELECT folder_path FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?)
    })?
    .flatten();

    if let Some(path_str) = &existing_path {
        let p = PathBuf::from(path_str);
        if p.exists() {
            let marker = p.join(MARKER_FILE);
            if !marker.exists() {
                let _ = fs::write(&marker, format!("{mal_id}\n"));
            }
            return Ok(p);
        }
    }

    // 2. Scan immediate subdirectories for .mal_id marker (detect folder rename)
    if library_folder.exists() {
        if let Ok(entries) = fs::read_dir(library_folder) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let marker = path.join(MARKER_FILE);
                    if marker.exists() {
                        if let Ok(content) = fs::read_to_string(&marker) {
                            if let Ok(id) = content.trim().parse::<i64>() {
                                if id == mal_id {
                                    db.with(|c| {
                                        c.execute(
                                            "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
                                            rusqlite::params![path.display().to_string(), mal_id],
                                        )?;
                                        Ok(())
                                    })?;
                                    return Ok(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 2b. If existing_path was set and not found in library_folder, it is an explicit override that must be created
    if let Some(path_str) = existing_path {
        let p = PathBuf::from(&path_str);
        fs::create_dir_all(&p)?;
        let marker = p.join(MARKER_FILE);
        if !marker.exists() {
            let _ = fs::write(&marker, format!("{mal_id}\n"));
        }
        return Ok(p);
    }

    // 3. Create a new show folder
    let clean_title = sanitize_component(title);
    let folder_name = match start_year {
        Some(year) => format!("{clean_title} ({year})"),
        None => clean_title,
    };

    let show_dir = library_folder.join(folder_name);
    fs::create_dir_all(&show_dir)?;
    let marker = show_dir.join(MARKER_FILE);
    fs::write(&marker, format!("{mal_id}\n"))?;

    db.with(|c| {
        c.execute(
            "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
            rusqlite::params![show_dir.display().to_string(), mal_id],
        )?;
        Ok(())
    })?;

    Ok(show_dir)
}

/// Rewrite synopsis.txt and movie.nfo/tvshow.nfo (plot, rating, year) for all existing folders of a show.
/// Called when shows.synopsis becomes ready or when metadata is updated.
pub fn rewrite_show_folder_metadata(db: &Db, mal_id: i64) -> Result<usize> {
    // 1. Fetch show details from database
    let show_data: Option<(
        String,
        Option<String>,
        String,
        Option<i64>,
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<i64>,
        Option<String>,
        Option<String>,
    )> = db.with(|c| {
        Ok(c.query_row(
            "SELECT title, english_title, type, start_year, synopsis, poster_path, score, total_episodes, status, folder_path
             FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                ))
            },
        )
        .optional()?)
    })?;

    let (title, english_title, show_type_str, start_year, synopsis, poster_path, score, total_episodes, status, folder_path) = match show_data {
        Some(d) => d,
        None => return Ok(0),
    };

    let syn = match synopsis {
        Some(s) if !s.trim().is_empty() && !is_placeholder_synopsis(&s) => s,
        _ => return Ok(0), // synopsis not ready yet
    };

    let show_type = ShowType::parse(&show_type_str);

    // Fetch Japanese title from show_names if present
    let japanese_title: Option<String> = db.with(|c| {
        Ok(c.query_row(
            "SELECT name FROM show_names WHERE mal_id = ?1 AND source = 'jikan' ORDER BY priority DESC LIMIT 1",
            [mal_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?)
    })?;

    // Load settings for library root
    let settings = Settings::load(db)?;
    let library_root = if settings.library_folder.trim().is_empty() {
        crate::locations::default_library_dir()
    } else {
        crate::locations::resolve_home_dir(&settings.library_folder)
    };

    // Collect all unique existing folders for this show
    let mut folders = HashSet::new();

    // 1) Folder path stored in shows table
    if let Some(ref fp) = folder_path {
        let p = PathBuf::from(fp);
        if p.is_dir() {
            folders.insert(p);
        }
    }

    // 2) Parent directory of any recorded episode file
    let ep_paths: Vec<String> = db.with(|c| {
        let mut stmt = c.prepare("SELECT DISTINCT file_path FROM episodes WHERE mal_id = ?1 AND file_path IS NOT NULL")?;
        let rows = stmt.query_map([mal_id], |r| r.get::<_, String>(0))?;
        let mut list = Vec::new();
        for r in rows.flatten() {
            if !r.trim().is_empty() {
                list.push(r);
            }
        }
        Ok(list)
    })?;

    for ep_p in ep_paths {
        let p = PathBuf::from(&ep_p);
        if let Some(parent) = p.parent() {
            if parent.is_dir() {
                folders.insert(parent.to_path_buf());
            }
        }
    }

    // 3) Any subfolder in library_root with matching .mal_id marker
    if library_root.is_dir() {
        if let Ok(entries) = fs::read_dir(&library_root) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    let marker = p.join(MARKER_FILE);
                    if marker.exists() {
                        if let Ok(content) = fs::read_to_string(&marker) {
                            if let Ok(id) = content.trim().parse::<i64>() {
                                if id == mal_id {
                                    folders.insert(p);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if folders.is_empty() {
        return Ok(0);
    }

    let now_date = format!(
        "Unix timestamp {}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    let syn_formatted = format_synopsis(
        &title,
        english_title.as_deref(),
        japanese_title.as_deref(),
        &show_type_str,
        total_episodes,
        status.as_deref(),
        None,
        score,
        &[],
        Some(&syn),
        mal_id,
        &now_date,
    );

    let tag = if show_type == ShowType::Movie { "movie" } else { "tvshow" };
    let nfo_content = format_nfo(
        tag,
        &title,
        english_title.as_deref(),
        &syn,
        score,
        start_year,
        mal_id,
    );

    let mut updated_count = 0;
    for dir in &folders {
        // Rewrite synopsis.txt
        let synopsis_file = dir.join(SYNOPSIS_FILE);
        let _ = fs::write(&synopsis_file, &syn_formatted);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&synopsis_file, fs::Permissions::from_mode(0o644));
        }

        // Rewrite movie.nfo or tvshow.nfo
        let movie_nfo = dir.join("movie.nfo");
        let tvshow_nfo = dir.join("tvshow.nfo");
        let target_nfo = if movie_nfo.exists() {
            movie_nfo
        } else if tvshow_nfo.exists() {
            tvshow_nfo
        } else if show_type == ShowType::Movie {
            movie_nfo
        } else {
            tvshow_nfo
        };

        let _ = fs::write(&target_nfo, &nfo_content);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target_nfo, fs::Permissions::from_mode(0o644));
        }

        // Ensure marker exists
        let marker_file = dir.join(MARKER_FILE);
        if !marker_file.exists() {
            let _ = fs::write(&marker_file, format!("{mal_id}\n"));
        }

        // If poster_path is a local file and folder.png is missing, create it
        let folder_png = dir.join(FOLDER_ICON_FILE);
        if !folder_png.exists() {
            if let Some(ref post) = poster_path {
                let pb = Path::new(post);
                if pb.is_file() {
                    let _ = create_folder_png(pb, &folder_png, settings.icon_style);
                }
            }
        }

        updated_count += 1;
    }

    Ok(updated_count)
}

// ---------------------------------------------------------------------------
// Library Rescan
// ---------------------------------------------------------------------------

/// Rescan the library root directory: adopts existing folders and indexes episodes.
pub fn rescan_library(db: &Db, library_folder: &Path) -> Result<RescanReport> {
    if !library_folder.exists() {
        return Ok(RescanReport::default());
    }

    let mut shows_adopted = 0usize;
    let mut episodes_found = 0usize;

    let entries = fs::read_dir(library_folder)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let marker = path.join(MARKER_FILE);
        let mut mal_id: Option<i64> = None;

        if marker.exists() {
            if let Ok(content) = fs::read_to_string(&marker) {
                if let Ok(id) = content.trim().parse::<i64>() {
                    mal_id = Some(id);
                }
            }
        }

        // If no marker, try matching by directory name against show titles
        if mal_id.is_none() {
            if let Some(dirname) = path.file_name().and_then(|n| n.to_str()) {
                let normalized = crate::names::normalize_alias(dirname);
                let matched_id: Option<i64> = db.with(|c| {
                    Ok(c.query_row(
                        "SELECT s.mal_id FROM shows s
                         LEFT JOIN show_names sn ON sn.mal_id = s.mal_id
                         WHERE sn.normalized = ?1 OR lower(s.title) = lower(?2)
                         LIMIT 1",
                        rusqlite::params![normalized, dirname],
                        |r| r.get(0),
                    )
                    .optional()?)
                })?;
                if let Some(id) = matched_id {
                    let _ = fs::write(&marker, format!("{id}\n"));
                    mal_id = Some(id);
                }
            }
        }

        if let Some(id) = mal_id {
            // Adopt show folder
            db.with(|c| {
                c.execute(
                    "UPDATE shows SET folder_path = ?1 WHERE mal_id = ?2",
                    rusqlite::params![path.display().to_string(), id],
                )?;
                Ok(())
            })?;
            shows_adopted += 1;

            // Scan video files for episodes
            if let Ok(files) = fs::read_dir(&path) {
                for file_entry in files.flatten() {
                    let file_path = file_entry.path();
                    if !file_path.is_file() || !is_video_file(&file_path) {
                        continue;
                    }
                    if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
                        let parsed = crate::filename::parse_release(filename);
                        if let Some(ep_num) = parsed.episode {
                            let size = fs::metadata(&file_path).map(|m| m.len() as i64).unwrap_or(0);
                            let updated = db.with(|c| {
                                Ok(c.execute(
                                    "UPDATE episodes SET file_path = ?1, size = ?2, state = 'done'
                                     WHERE mal_id = ?3 AND number = ?4",
                                    rusqlite::params![file_path.display().to_string(), size, id, ep_num],
                                )?)
                            })?;
                            if updated > 0 {
                                episodes_found += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(RescanReport {
        shows_adopted,
        episodes_found,
    })
}

// ---------------------------------------------------------------------------
// LibraryFinalizer
// ---------------------------------------------------------------------------

/// The production Finalizer that renames, moves, and decorates completed downloads.
pub struct LibraryFinalizer {
    db: Db,
    gate: Arc<Gate>,
    clock: Arc<dyn Clock>,
    data_dir: PathBuf,
}

impl LibraryFinalizer {
    pub fn new(
        db: Db,
        gate: Arc<Gate>,
        clock: Arc<dyn Clock>,
        data_dir: PathBuf,
    ) -> Self {
        LibraryFinalizer {
            db,
            gate,
            clock,
            data_dir,
        }
    }

    /// Retrieve Jikan metadata with fallback to shows table.
    async fn fetch_metadata(
        &self,
        mal_id: i64,
        settings: &Settings,
    ) -> Option<JikanAnimeData> {
        let url = format!("{}/anime/{}", settings.jikan_base_url.trim_end_matches('/'), mal_id);
        let opts = GetOptions {
            ttl: Some(Duration::from_secs(settings.jikan_cache_secs as u64)),
            ..Default::default()
        };

        match self.gate.get_with(Service::Jikan, &url, &opts).await {
            Ok(fetched) => match fetched.json::<JikanResponse>() {
                Ok(resp) => Some(resp.data),
                Err(err) => {
                    tracing::warn!(mal_id, "failed to parse Jikan metadata: {err}");
                    None
                }
            },
            Err(err) => {
                tracing::warn!(mal_id, "failed to fetch Jikan metadata: {err}");
                None
            }
        }
    }

    /// Download and save the show poster locally.
    async fn download_poster(&self, mal_id: i64, url: &str) -> Option<PathBuf> {
        let poster_dir = self.data_dir.join("posters");
        let _ = fs::create_dir_all(&poster_dir);
        let dst = poster_dir.join(format!("{mal_id}.jpg"));

        if dst.exists() {
            return Some(dst);
        }

        match self.gate.get_with(Service::Images, url, &GetOptions::default()).await {
            Ok(fetched) => {
                if fs::write(&dst, &fetched.body).is_ok() {
                    Some(dst)
                } else {
                    None
                }
            }
            Err(err) => {
                tracing::warn!(mal_id, url, "failed to download poster: {err}");
                None
            }
        }
    }

    /// Finalize a batch directory containing multiple episode video files.
    async fn finalize_batch(
        &self,
        context: &FinalizeContext<'_>,
        batch_dir: &Path,
        settings: &Settings,
    ) -> Result<FinalizeOutcome> {
        let library_root = if settings.library_folder.trim().is_empty() {
            crate::locations::default_library_dir()
        } else {
            crate::locations::resolve_home_dir(&settings.library_folder)
        };
        if !library_root.exists() {
            let _ = fs::create_dir_all(&library_root);
        }

        let (title, english_title, show_type_str, start_year, season, existing_synopsis, existing_poster, existing_score, existing_episodes, existing_status) = self.db.with(|c| {
            Ok(c.query_row(
                "SELECT title, english_title, type, start_year, season, synopsis, poster_path, score, total_episodes, status FROM shows WHERE mal_id = ?1",
                [context.mal_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, Option<String>>(5)?,
                        r.get::<_, Option<String>>(6)?,
                        r.get::<_, Option<f64>>(7)?,
                        r.get::<_, Option<i64>>(8)?,
                        r.get::<_, Option<String>>(9)?,
                    ))
                },
            )?)
        })?;
        let show_type = ShowType::parse(&show_type_str);

        let show_dir = resolve_show_folder(
            &self.db,
            &library_root,
            context.mal_id,
            &title,
            start_year,
        )?;

        // Ensure synopsis, NFO, and folder icon
        let synopsis_path = show_dir.join(SYNOPSIS_FILE);
        let folder_png_path = show_dir.join(FOLDER_ICON_FILE);
        let nfo_path = if show_type == ShowType::Movie {
            show_dir.join("movie.nfo")
        } else {
            show_dir.join("tvshow.nfo")
        };

        if !synopsis_path.exists() || !folder_png_path.exists() || !nfo_path.exists() {
            let meta = if existing_synopsis.is_none() || existing_poster.is_none() {
                self.fetch_metadata(context.mal_id, settings).await
            } else {
                None
            };
            let now_date = format!("Unix timestamp {}", self.clock.now());
            let syn_text = match (&meta, &existing_synopsis) {
                (Some(m), _) => {
                    let genres: Vec<String> = m.genres.iter().map(|g| g.name.clone()).collect();
                    format_synopsis(
                        m.title.as_deref().unwrap_or(&title),
                        m.title_english.as_deref().or(english_title.as_deref()),
                        m.title_japanese.as_deref(),
                        m.show_type.as_deref().unwrap_or(&show_type_str),
                        m.episodes.or(existing_episodes),
                        m.status.as_deref().or(existing_status.as_deref()),
                        m.aired.as_ref().and_then(|a| a.string.as_deref()),
                        m.score.or(existing_score),
                        &genres,
                        m.synopsis.as_deref().or(existing_synopsis.as_deref()),
                        context.mal_id,
                        &now_date,
                    )
                }
                (None, Some(syn)) => format_synopsis(
                    &title,
                    english_title.as_deref(),
                    None,
                    &show_type_str,
                    existing_episodes,
                    existing_status.as_deref(),
                    None,
                    existing_score,
                    &[],
                    Some(syn.as_str()),
                    context.mal_id,
                    &now_date,
                ),
                (None, None) => format_synopsis(
                    &title,
                    english_title.as_deref(),
                    None,
                    &show_type_str,
                    existing_episodes,
                    existing_status.as_deref(),
                    None,
                    existing_score,
                    &[],
                    None,
                    context.mal_id,
                    &now_date,
                ),
            };
            let should_write_syn = !synopsis_path.exists() || {
                if let Ok(c) = fs::read_to_string(&synopsis_path) {
                    is_placeholder_synopsis(&c) && !is_placeholder_synopsis(&syn_text)
                } else {
                    false
                }
            };
            if should_write_syn {
                let _ = fs::write(&synopsis_path, &syn_text);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&synopsis_path, fs::Permissions::from_mode(0o644));
                }
            }

            let plot = meta.as_ref().and_then(|m| m.synopsis.as_deref())
                .or(existing_synopsis.as_deref())
                .unwrap_or("");
            let should_write_nfo = !nfo_path.exists() || {
                if let Ok(c) = fs::read_to_string(&nfo_path) {
                    (c.contains("<plot></plot>") || c.contains("<plot/>")) && !plot.trim().is_empty()
                } else {
                    false
                }
            };
            if should_write_nfo {
                let tag = if show_type == ShowType::Movie { "movie" } else { "tvshow" };
                let score_val = meta.as_ref().and_then(|m| m.score).or(existing_score);
                let nfo_content = format_nfo(
                    tag,
                    &title,
                    english_title.as_deref(),
                    plot,
                    score_val,
                    start_year,
                    context.mal_id,
                );
                let _ = fs::write(&nfo_path, nfo_content);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&nfo_path, fs::Permissions::from_mode(0o644));
                }
            }

            if !folder_png_path.exists() {
                let poster_url = meta.as_ref().and_then(|m| {
                    m.images.as_ref().and_then(|imgs| {
                        imgs.jpg.as_ref().and_then(|j| j.large_image_url.clone().or_else(|| j.image_url.clone()))
                            .or_else(|| imgs.webp.as_ref().and_then(|w| w.large_image_url.clone().or_else(|| w.image_url.clone())))
                    })
                }).or_else(|| {
                    existing_poster.as_ref().and_then(|p| {
                        if p.starts_with("http://") || p.starts_with("https://") {
                            Some(p.clone())
                        } else {
                            None
                        }
                    })
                });

                let master_poster = match poster_url {
                    Some(url) => self.download_poster(context.mal_id, &url).await,
                    None => {
                        if let Some(ref local_p) = existing_poster {
                            let pb = PathBuf::from(local_p);
                            if pb.exists() {
                                Some(pb)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                };

                let poster_to_use = master_poster.unwrap_or_else(|| {
                    let placeholder = self.data_dir.join("placeholder_poster.png");
                    if !placeholder.exists() {
                        let _ = write_solid_png(&placeholder, 64, 64, 30, 20, 50);
                    }
                    placeholder
                });
                let _ = create_folder_png(&poster_to_use, &folder_png_path, settings.icon_style);
                let _ = apply_folder_icon(settings.folder_icon_method, &show_dir, &folder_png_path);
            }
        }

        // Collect all video files recursively in batch_dir
        let mut video_files = Vec::new();
        let mut to_visit = vec![batch_dir.to_path_buf()];
        while let Some(dir) = to_visit.pop() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        to_visit.push(path);
                    } else if path.is_file() {
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                            video_files.push(path);
                        }
                    }
                }
            }
        }

        video_files.sort();
        let mut primary_dest = String::new();

        for file_path in video_files {
            let fname = file_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            let parsed = crate::filename::parse_release(fname);
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mkv")
                .to_ascii_lowercase();

            let ep_num = parsed.episode.unwrap_or(1.0);
            let target_filename = if show_type == ShowType::Movie {
                format_episode_filename(
                    &settings.movie_naming_pattern,
                    &title,
                    season,
                    ep_num,
                    start_year,
                    &ext,
                )
            } else {
                format_episode_filename(
                    &settings.naming_pattern,
                    &title,
                    season,
                    ep_num,
                    start_year,
                    &ext,
                )
            };

            let target_path = show_dir.join(&target_filename);
            let file_size = fs::metadata(&file_path).map(|m| m.len() as i64).unwrap_or(0);
            if !target_path.exists() {
                let _ = transfer_file(&file_path, &target_path, settings.move_mode);
            }

            let final_dest = target_path.display().to_string();
            if primary_dest.is_empty() {
                primary_dest = final_dest.clone();
            }

            // Update database episode row
            let _ = self.db.with(|c| {
                c.execute(
                    "INSERT INTO episodes (mal_id, number, state, file_path, size)
                     VALUES (?1, ?2, 'done', ?3, ?4)
                     ON CONFLICT (mal_id, number) DO UPDATE SET
                         state = 'done',
                         file_path = excluded.file_path,
                         size = excluded.size",
                    rusqlite::params![context.mal_id, ep_num, final_dest, file_size],
                )?;
                Ok(())
            });
        }

        if primary_dest.is_empty() {
            primary_dest = show_dir.display().to_string();
        }

        Ok(FinalizeOutcome::Done(FinalizeSuccess {
            destination_path: primary_dest,
        }))
    }
}

impl Finalizer for LibraryFinalizer {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>> {
        Box::pin(async move {
            let settings = Settings::load(&self.db)?;
            let source_path = Path::new(context.content_path);
            if !source_path.exists() {
                return Err(Error::Other(format!(
                    "finished content path {} does not exist on disk",
                    context.content_path
                )));
            }

            if source_path.is_dir() {
                return self.finalize_batch(context, source_path, &settings).await;
            }

            // Verify extension allowlist
            let ext = source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                return Err(Error::Other(format!(
                    "file extension .{} is not an allowed video format",
                    ext
                )));
            }

            let library_root = if settings.library_folder.trim().is_empty() {
                crate::locations::default_library_dir()
            } else {
                crate::locations::resolve_home_dir(&settings.library_folder)
            };
            if !library_root.exists() {
                let _ = fs::create_dir_all(&library_root);
            }

            // Load show information from database
            let (title, english_title, show_type_str, start_year, season, existing_synopsis, existing_poster, existing_score, existing_episodes, existing_status) = self.db.with(|c| {
                Ok(c.query_row(
                    "SELECT title, english_title, type, start_year, season, synopsis, poster_path, score, total_episodes, status FROM shows WHERE mal_id = ?1",
                    [context.mal_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Option<i64>>(3)?,
                            r.get::<_, i64>(4)?,
                            r.get::<_, Option<String>>(5)?,
                            r.get::<_, Option<String>>(6)?,
                            r.get::<_, Option<f64>>(7)?,
                            r.get::<_, Option<i64>>(8)?,
                            r.get::<_, Option<String>>(9)?,
                        ))
                    },
                )?)
            })?;
            let show_type = ShowType::parse(&show_type_str);

            // 1. Resolve Show Folder
            let show_dir = resolve_show_folder(
                &self.db,
                &library_root,
                context.mal_id,
                &title,
                start_year,
            )?;

            // 2. Write synopsis.txt, NFO, and folder.png if not present
            let synopsis_path = show_dir.join(SYNOPSIS_FILE);
            let folder_png_path = show_dir.join(FOLDER_ICON_FILE);
            let nfo_path = if show_type == ShowType::Movie {
                show_dir.join("movie.nfo")
            } else {
                show_dir.join("tvshow.nfo")
            };

            if !synopsis_path.exists() || !folder_png_path.exists() || !nfo_path.exists() {
                let meta = if existing_synopsis.is_none() || existing_poster.is_none() {
                    self.fetch_metadata(context.mal_id, &settings).await
                } else {
                    None
                };
                let now_date = {
                    let ts = self.clock.now();
                    format!("Unix timestamp {ts}")
                };

                let syn_text = match (&meta, &existing_synopsis) {
                    (Some(m), _) => {
                        let genres: Vec<String> = m.genres.iter().map(|g| g.name.clone()).collect();
                        format_synopsis(
                            m.title.as_deref().unwrap_or(&title),
                            m.title_english.as_deref().or(english_title.as_deref()),
                            m.title_japanese.as_deref(),
                            m.show_type.as_deref().unwrap_or(&show_type_str),
                            m.episodes.or(existing_episodes),
                            m.status.as_deref().or(existing_status.as_deref()),
                            m.aired.as_ref().and_then(|a| a.string.as_deref()),
                            m.score.or(existing_score),
                            &genres,
                            m.synopsis.as_deref().or(existing_synopsis.as_deref()),
                            context.mal_id,
                            &now_date,
                        )
                    }
                    (None, Some(syn)) => format_synopsis(
                        &title,
                        english_title.as_deref(),
                        None,
                        &show_type_str,
                        existing_episodes,
                        existing_status.as_deref(),
                        None,
                        existing_score,
                        &[],
                        Some(syn.as_str()),
                        context.mal_id,
                        &now_date,
                    ),
                    (None, None) => format_synopsis(
                        &title,
                        english_title.as_deref(),
                        None,
                        &show_type_str,
                        existing_episodes,
                        existing_status.as_deref(),
                        None,
                        existing_score,
                        &[],
                        None,
                        context.mal_id,
                        &now_date,
                    ),
                };

                let should_write_syn = !synopsis_path.exists() || {
                    if let Ok(c) = fs::read_to_string(&synopsis_path) {
                        is_placeholder_synopsis(&c) && !is_placeholder_synopsis(&syn_text)
                    } else {
                        false
                    }
                };
                if should_write_syn {
                    let _ = fs::write(&synopsis_path, &syn_text);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&synopsis_path, fs::Permissions::from_mode(0o644));
                    }
                }

                // NFO metadata file
                let plot = meta.as_ref().and_then(|m| m.synopsis.as_deref())
                    .or(existing_synopsis.as_deref())
                    .unwrap_or("");
                let should_write_nfo = !nfo_path.exists() || {
                    if let Ok(c) = fs::read_to_string(&nfo_path) {
                        (c.contains("<plot></plot>") || c.contains("<plot/>")) && !plot.trim().is_empty()
                    } else {
                        false
                    }
                };
                if should_write_nfo {
                    let tag = if show_type == ShowType::Movie { "movie" } else { "tvshow" };
                    let score_val = meta.as_ref().and_then(|m| m.score).or(existing_score);
                    let nfo_content = format_nfo(
                        tag,
                        &title,
                        english_title.as_deref(),
                        plot,
                        score_val,
                        start_year,
                        context.mal_id,
                    );
                    let _ = fs::write(&nfo_path, nfo_content);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&nfo_path, fs::Permissions::from_mode(0o644));
                    }
                }

                // Poster & folder icon
                if !folder_png_path.exists() {
                    let poster_url = meta.as_ref().and_then(|m| {
                        m.images.as_ref().and_then(|imgs| {
                            imgs.jpg.as_ref().and_then(|j| j.large_image_url.clone().or_else(|| j.image_url.clone()))
                                .or_else(|| imgs.webp.as_ref().and_then(|w| w.large_image_url.clone().or_else(|| w.image_url.clone())))
                        })
                    }).or_else(|| {
                        existing_poster.as_ref().and_then(|p| {
                            if p.starts_with("http://") || p.starts_with("https://") {
                                Some(p.clone())
                            } else {
                                None
                            }
                        })
                    });

                    let master_poster = match poster_url {
                        Some(url) => self.download_poster(context.mal_id, &url).await,
                        None => {
                            if let Some(ref local_p) = existing_poster {
                                let pb = PathBuf::from(local_p);
                                if pb.exists() {
                                    Some(pb)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    };

                    let poster_to_use = master_poster.unwrap_or_else(|| {
                        let placeholder = self.data_dir.join("placeholder_poster.png");
                        if !placeholder.exists() {
                            let _ = write_solid_png(&placeholder, 64, 64, 30, 20, 50);
                        }
                        placeholder
                    });

                    let _ = create_folder_png(&poster_to_use, &folder_png_path, settings.icon_style);
                    let _ = apply_folder_icon(settings.folder_icon_method, &show_dir, &folder_png_path);
                }
            }

            // 3. Determine Episode Filename
            let ep_number = self.db.with(|c| {
                Ok(c.query_row(
                    "SELECT number FROM episodes WHERE id = ?1",
                    [context.episode_id],
                    |r| r.get::<_, f64>(0),
                )?)
            })?;

            let filename = if show_type == ShowType::Movie {
                format_episode_filename(
                    &settings.movie_naming_pattern,
                    &title,
                    season,
                    ep_number,
                    start_year,
                    &ext,
                )
            } else {
                format_episode_filename(
                    &settings.naming_pattern,
                    &title,
                    season,
                    ep_number,
                    start_year,
                    &ext,
                )
            };

            let target_path = show_dir.join(&filename);

            // Ensure destination sits inside library root (Path traversal check)
            let can_target = target_path.canonicalize().unwrap_or_else(|_| target_path.clone());
            if let Ok(can_root) = library_root.canonicalize() {
                if !can_target.starts_with(&can_root) {
                    return Err(Error::Other(format!(
                        "path traversal detected: target '{}' is outside library root '{}'",
                        target_path.display(),
                        library_root.display()
                    )));
                }
            }

            // 4. Collision check
            if target_path.exists() {
                // If identical file (same inode/hardlink or same path), already filed
                if let (Ok(m1), Ok(m2)) = (fs::metadata(source_path), fs::metadata(&target_path)) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if m1.dev() == m2.dev() && m1.ino() == m2.ino() {
                            return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                destination_path: target_path.display().to_string(),
                            }));
                        }
                    }
                }

                // Check questions table for an existing answer
                let question: Option<(i64, Option<String>)> = self.db.with(|c| {
                    Ok(c.query_row(
                        "SELECT id, answer FROM questions WHERE job_id = ?1 AND kind = 'file_exists'",
                        [context.job.id],
                        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
                    )
                    .optional()?)
                })?;

                match question {
                    Some((_qid, Some(ans_json))) => {
                        let answer: Answer = serde_json::from_str(&ans_json)
                            .unwrap_or(Answer { choice: Choice::Replace, always: false });
                        match answer.choice {
                            Choice::Keep => {
                                // Keep existing file on disk as the episode file
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: target_path.display().to_string(),
                                }));
                            }
                            Choice::Replace => {
                                let _ = fs::remove_file(&target_path);
                                transfer_file(source_path, &target_path, settings.move_mode)?;
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: target_path.display().to_string(),
                                }));
                            }
                            Choice::KeepBoth => {
                                let stem = target_path.file_stem().and_then(|s| s.to_str()).unwrap_or("episode");
                                let alt_filename = format!("{stem} (v2).{ext}");
                                let alt_path = show_dir.join(alt_filename);
                                transfer_file(source_path, &alt_path, settings.move_mode)?;
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: alt_path.display().to_string(),
                                }));
                            }
                            _ => {}
                        }
                    }
                    Some((qid, None)) => {
                        // Already asked, still awaiting answer
                        return Ok(FinalizeOutcome::NeedsInput { question_id: qid });
                    }
                    None => {
                        // Ask question in Inbox
                        let payload = Payload::FileExists {
                            episode: ep_number,
                            existing: target_path.display().to_string(),
                            candidate: context.content_path.to_string(),
                        };
                        let now = self.clock.now();
                        let qid = QuestionStore::ask(
                            &self.db,
                            Some(context.job.id),
                            Some(context.mal_id),
                            &payload,
                            now,
                            Some(i64::from(settings.question_expiry_hours) * 3600),
                        )?;
                        EventLog::new(self.db.clone()).warn(
                            Some(context.job.id),
                            Some(context.mal_id),
                            format!("destination file exists: asked question #{qid}"),
                        )?;
                        return Ok(FinalizeOutcome::NeedsInput { question_id: qid });
                    }
                }
            }

            // 5. Transfer file into show folder
            transfer_file(source_path, &target_path, settings.move_mode)?;

            Ok(FinalizeOutcome::Done(FinalizeSuccess {
                destination_path: target_path.display().to_string(),
            }))
        })
    }
}
