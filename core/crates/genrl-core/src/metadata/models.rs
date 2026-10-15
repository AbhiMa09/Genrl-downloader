use serde::Deserialize;

// ---------------------------------------------------------------------------
// Jikan API Wire Models (/v4/anime/{id}/full)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullResponse {
    pub data: JikanFullData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullData {
    pub mal_id: i64,
    pub title: String,
    #[serde(default)]
    pub title_english: Option<String>,
    #[serde(default)]
    pub title_japanese: Option<String>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(rename = "type", default)]
    pub show_type: Option<String>,
    #[serde(default)]
    pub episodes: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub aired: Option<JikanFullAired>,
    #[serde(default)]
    pub images: Option<JikanFullImages>,
    #[serde(default)]
    pub genres: Vec<JikanNamedItem>,
    #[serde(default)]
    pub studios: Vec<JikanNamedItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullAired {
    #[serde(default)]
    pub prop: Option<JikanAiredProp>,
    #[serde(default)]
    pub string: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAiredProp {
    #[serde(default)]
    pub from: Option<JikanAiredDate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAiredDate {
    #[serde(default)]
    pub year: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullImages {
    #[serde(default)]
    pub jpg: Option<JikanImageSet>,
    #[serde(default)]
    pub webp: Option<JikanImageSet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanImageSet {
    #[serde(default)]
    pub large_image_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanNamedItem {
    pub name: String,
}

// ---------------------------------------------------------------------------
// MAL API Fallback Wire Models (/v2/anime/{id})
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiAnimeDetailResponse {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub alternative_titles: Option<MalApiAlternativeTitles>,
    #[serde(default)]
    pub main_picture: Option<MalApiPicture>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub mean: Option<f64>,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub num_episodes: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub start_season: Option<MalApiSeason>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiAlternativeTitles {
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub ja: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiPicture {
    #[serde(default)]
    pub large: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiSeason {
    #[serde(default)]
    pub year: Option<i64>,
}

// ---------------------------------------------------------------------------
// Sanitization Helpers
// ---------------------------------------------------------------------------

/// Strip control characters and ANSI escape sequences from text.
/// Preserves standard newlines (\n, \r) and tabs (\t).
pub fn clean_plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        // ASCII control chars below 32 except \n (10), \r (13), \t (9)
        if (c as u32) < 32 && c != '\n' && c != '\r' && c != '\t' {
            continue;
        }
        // DEL (0x7f) and zero-width spaces/marks
        if c == '\x7f' || c == '\u{feff}' || c == '\u{200b}' {
            continue;
        }
        out.push(c);
    }

    out.trim().to_string()
}

/// Strip all control characters AND newlines for single-line headers or filenames.
pub fn clean_single_line(s: &str) -> String {
    clean_plain_text(s)
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate backoff in seconds based on retry attempt count.
pub fn calculate_backoff(retry_count: u32) -> i64 {
    match retry_count {
        0 | 1 => 30,
        2 => 60,
        3 => 120,
        4 => 300,
        5 => 600,
        _ => 1800,
    }
}

pub fn placeholder_synopsis() -> &'static str {
    "[Synopsis loading... could not fetch metadata, will retry]"
}
