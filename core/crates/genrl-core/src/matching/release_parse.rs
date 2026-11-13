use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

/// Structured metadata parsed from a movie or TV drama release name.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MediaRelease {
    pub raw: String,
    pub title: String,
    pub normalized_title: String,
    pub year: Option<u32>,
    pub season: Option<u32>,
    pub episode: Option<f64>,
    pub season_range: Option<(u32, u32)>,
    pub episode_range: Option<(f64, f64)>,
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub codec: Option<String>,
    pub audio: Option<String>,
    pub group: Option<String>,
    pub is_batch: bool,
    pub is_cam: bool,
    pub is_proper: bool,
    pub is_repack: bool,
    pub is_hardcoded_subs: bool,
}

static YEAR_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(19\d\d|20[0-2]\d|2030)\b").unwrap());
static S_RANGE_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bS(\d{1,2})[-~]S?(\d{1,2})\b").unwrap());
static SE_RANGE_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})[-~]E?(\d{1,3})\b").unwrap());
static SE_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bS(\d{1,2})E(\d{1,3})\b").unwrap());
static X_EP_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})x(\d{1,3})\b").unwrap());
static EP_ONLY_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bE(\d{1,3})\b").unwrap());
static SEASON_WORD_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bSeason[\s._]*(\d{1,2})\b").unwrap());
static S_ONLY_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bS(\d{1,2})([.\s_-]|$)").unwrap());
static RES_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(2160p|1080p|720p|480p|4k|uhd)\b").unwrap());
static CAM_RX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(camrip|hdcam|hd-ts|hdts|telesync|ts|tc|telecine|dvdscr|scr|cam)\b").unwrap()
});
static CAM_TS_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(ts|hd-ts|hdts)\b").unwrap());
static CAM_TC_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(tc|telecine)\b").unwrap());
static CAM_SCR_RX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(dvdscr|scr)\b").unwrap());
static HARDSUB_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(korsub|hc|hardsub|hardsubs|hardcoded)\b").unwrap());
static DUAL_AUDIO_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(dual[\s._-]?audio|multi[\s._-]?audio)\b").unwrap());
static CODEC_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(x265|h\.?265|hevc|x264|h\.?264|avc|av1)\b").unwrap());
static AUDIO_RX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(atmos|truehd|dts-hd|dts|aac|ac3)(?:5\.1|\b)").unwrap());

/// Parse a raw torrent or file name into a structured MediaRelease.
pub fn parse_media_release(raw_input: &str) -> MediaRelease {
    let raw = raw_input.trim();
    let mut clean = raw.to_string();

    // 1. Remove file extension (.mkv, .mp4, etc.)
    if let Some((stem, ext)) = clean.rsplit_once('.') {
        let ext_lower = ext.to_ascii_lowercase();
        if ["mkv", "mp4", "avi", "mov", "ts", "m4v"].contains(&ext_lower.as_str()) {
            clean = stem.to_string();
        }
    }

    // 2. Extract group: trailing -GROUP, or leading [GROUP]
    let mut group = None;
    if clean.starts_with('[') {
        if let Some(idx) = clean.find(']') {
            let candidate = &clean[1..idx];
            if !candidate.is_empty() {
                group = Some(candidate.to_string());
                clean = clean[idx + 1..].trim().to_string();
            }
        }
    }
    if group.is_none() {
        if let Some((_, grp)) = clean.rsplit_once('-') {
            let grp = grp.trim().trim_matches(|c| c == '[' || c == ']');
            if !grp.is_empty() && !grp.contains(' ') && grp.len() <= 20 {
                group = Some(grp.to_string());
            }
        }
    }

    // 3. Flags: CAM, Proper, Repack, Hardcoded subs
    let is_cam = CAM_RX.is_match(&clean);
    let is_proper = clean.to_ascii_uppercase().contains("PROPER");
    let is_repack = clean.to_ascii_uppercase().contains("REPACK");
    let is_hardcoded_subs = HARDSUB_RX.is_match(&clean);

    // 4. Source
    let lower_clean = clean.to_ascii_lowercase();
    let source = if lower_clean.contains("remux") {
        Some("remux".to_string())
    } else if lower_clean.contains("bluray") || lower_clean.contains("blu-ray") {
        Some("bluray".to_string())
    } else if lower_clean.contains("web-dl") || lower_clean.contains("webdl") {
        Some("web-dl".to_string())
    } else if lower_clean.contains("webrip") {
        Some("webrip".to_string())
    } else if lower_clean.contains("hdtv") {
        Some("hdtv".to_string())
    } else if lower_clean.contains("camrip") {
        Some("camrip".to_string())
    } else if lower_clean.contains("hdcam") {
        Some("hdcam".to_string())
    } else if lower_clean.contains("hd-ts") || lower_clean.contains("hdts") {
        Some("hdts".to_string())
    } else if lower_clean.contains("telesync") {
        Some("telesync".to_string())
    } else if lower_clean.contains("telecine") {
        Some("telecine".to_string())
    } else if CAM_SCR_RX.is_match(&clean) {
        Some("scr".to_string())
    } else if CAM_TC_RX.is_match(&clean) {
        Some("tc".to_string())
    } else if CAM_TS_RX.is_match(&clean) {
        Some("ts".to_string())
    } else if lower_clean.contains("cam") {
        Some("cam".to_string())
    } else if lower_clean.contains("hdrip") {
        Some("hdrip".to_string())
    } else if lower_clean.contains("web") {
        Some("web".to_string())
    } else {
        None
    };

    // 5. Resolution
    let resolution = RES_RX.captures(&clean).map(|c| {
        let val = c[1].to_ascii_lowercase();
        if val == "4k" || val == "uhd" {
            "2160p".to_string()
        } else {
            val
        }
    });

    // 6. Codec
    let codec = CODEC_RX.captures(&clean).map(|c| {
        let v = c[1].to_ascii_lowercase().replace('.', "");
        match v.as_str() {
            "avc" => "h264".to_string(),
            other => other.to_string(),
        }
    });

    // 7. Audio
    let audio = if DUAL_AUDIO_RX.is_match(&clean) {
        Some("dual-audio".to_string())
    } else {
        AUDIO_RX.captures(&clean).map(|c| c[1].to_ascii_lowercase())
    };

    // 8. Season, Episode, Batch
    let mut season = None;
    let mut episode = None;
    let mut season_range = None;
    let mut episode_range = None;
    let mut is_batch = lower_clean.contains("complete") || lower_clean.contains("batch");

    if let Some(caps) = S_RANGE_RX.captures(&clean) {
        if let (Ok(s1), Ok(s2)) = (caps[1].parse::<u32>(), caps[2].parse::<u32>()) {
            season_range = Some((s1, s2));
            is_batch = true;
        }
    } else if let Some(caps) = SE_RANGE_RX.captures(&clean) {
        season = caps[1].parse::<u32>().ok();
        if let (Ok(e1), Ok(e2)) = (caps[2].parse::<f64>(), caps[3].parse::<f64>()) {
            episode_range = Some((e1, e2));
        }
    } else if let Some(caps) = SE_RX.captures(&clean) {
        season = caps[1].parse::<u32>().ok();
        episode = caps[2].parse::<f64>().ok();
    } else if let Some(caps) = X_EP_RX.captures(&clean) {
        season = caps[1].parse::<u32>().ok();
        episode = caps[2].parse::<f64>().ok();
    } else if let Some(caps) = SEASON_WORD_RX.captures(&clean) {
        season = caps[1].parse::<u32>().ok();
        is_batch = true;
    } else if let Some(caps) = S_ONLY_RX.captures(&clean) {
        season = caps[1].parse::<u32>().ok();
        is_batch = true;
    } else if let Some(caps) = EP_ONLY_RX.captures(&clean) {
        episode = caps[1].parse::<f64>().ok();
    }

    // 9. Year and Title boundary cut
    let mut year = None;
    let mut boundary = clean.len();

    let mut tag_boundary = clean.len();
    if let Some(m) = S_RANGE_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = SE_RANGE_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = SE_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = X_EP_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = SEASON_WORD_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = S_ONLY_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = EP_ONLY_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = RES_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }
    if let Some(m) = CAM_RX.find(&clean) {
        tag_boundary = tag_boundary.min(m.start());
    }

    let year_matches: Vec<_> = YEAR_RX.find_iter(&clean[..tag_boundary]).collect();
    if year_matches.len() > 1 {
        let last_m = year_matches.last().unwrap();
        if let Ok(y) = last_m.as_str().parse::<u32>() {
            year = Some(y);
            boundary = boundary.min(last_m.start());
        }
    } else if let Some(m) = year_matches.first() {
        if let Ok(y) = m.as_str().parse::<u32>() {
            let after_year = clean[m.end()..tag_boundary]
                .trim_matches(|c: char| c == '.' || c == ' ' || c == '_' || c == '-');
            if m.start() == 0 && !after_year.is_empty() {
                // e.g. "2001.A.Space.Odyssey..." with no second year: part of title
            } else {
                year = Some(y);
                boundary = boundary.min(m.start());
            }
        }
    }

    boundary = boundary.min(tag_boundary);

    // Anime dash marker: "Title - 28"
    if episode.is_none() {
        if let Some((_t_part, ep_part)) = clean[..boundary].rsplit_once(" - ") {
            if let Some(ep_str) = ep_part.split_whitespace().next() {
                if let Ok(ep_num) = ep_str.parse::<f64>() {
                    episode = Some(ep_num);
                    boundary = clean.find(" - ").unwrap_or(boundary);
                }
            }
        }
    }

    let title_slice = &clean[..boundary];
    let title = title_slice
        .replace(['.', '_', '+'], " ")
        .replace(['(', ')', '[', ']'], "")
        .trim()
        .to_string();

    let normalized_title = title
        .to_ascii_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "");

    MediaRelease {
        raw: raw.to_string(),
        title,
        normalized_title,
        year,
        season,
        episode,
        season_range,
        episode_range,
        resolution,
        source,
        codec,
        audio,
        group,
        is_batch,
        is_cam,
        is_proper,
        is_repack,
        is_hardcoded_subs,
    }
}
