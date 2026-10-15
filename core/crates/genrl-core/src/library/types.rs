use serde::{Deserialize, Serialize};

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
pub(crate) struct JikanResponse {
    pub(crate) data: JikanAnimeData,
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
