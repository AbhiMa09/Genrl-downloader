use std::sync::OnceLock;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// What a release name tells us.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Release {
    /// The name exactly as it came in.
    pub raw: String,
    /// Fansub group from the leading `[...]`, e.g. `SubsPlease`.
    pub group: Option<String>,
    /// Title with group, CRC, resolution and other tags removed.
    pub title: String,
    /// Normalized form of `title`, ready to compare against `show_names`.
    pub normalized_title: String,
    pub season: Option<u32>,
    /// `None` for movies and batches; `6.5` for OVA-style decimals.
    pub episode: Option<f64>,
    pub resolution: Option<String>,
    /// `hevc`, `avc`, `av1`, …
    pub codec: Option<String>,
    /// `bd`, `remux`, `webrip`, `web-dl`, `tv`.
    pub source: Option<String>,
    /// A batch: `01-12`, "batch", "complete".
    pub is_batch: bool,
    pub dual_audio: bool,
    /// Subtitle/audio languages seen in the metadata brackets.
    pub languages: Vec<String>,
    /// The `[ABCD1234]` checksum, if present.
    pub crc: Option<String>,
    /// `v2` — a fixed re-release.
    pub version: Option<u32>,
    /// Year in the release name (remakes with the same title differ here).
    pub year: Option<i64>,
    /// File extension when the name is a filename, lower-cased without the dot.
    pub extension: Option<String>,
    /// Where the episode token started in `raw`, so the title can be cut there.
    #[serde(skip)]
    pub marker_start: Option<usize>,
}

pub fn re(pattern: &str) -> &'static Regex {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static CACHE: OnceLock<Mutex<HashMap<String, &'static Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(rx) = map.get(pattern) {
        return rx;
    }
    let rx = Box::leak(Box::new(Regex::new(pattern).expect("static regex is valid")));
    map.insert(pattern.to_string(), rx);
    rx
}

pub const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m2ts", "ts", "wmv", "mov", "webm"];

pub fn extract_season_episode(name: &str, release: &mut Release) -> Option<usize> {
    // `S01E05` / `S1E5`
    if let Some(caps) = re(r"(?i)\bS(\d{1,2})\s*[- ]?\s*E(\d{1,3})\b").captures(name) {
        release.season = caps[1].parse().ok();
        release.episode = caps[2].parse().ok();
        return Some(caps.get(0).unwrap().start());
    }

    // `01-12` range: a batch, and deliberately not an episode.
    if let Some(caps) = re(r"(?:^|[\s\[(])(\d{1,3})\s*-\s*(\d{1,3})(?:\s|[\])]|$)").captures(name) {
        let from: f64 = caps[1].parse().unwrap_or(0.0);
        let to: f64 = caps[2].parse().unwrap_or(0.0);
        if to > from {
            release.is_batch = true;
            return Some(caps.get(0).unwrap().start());
        }
    }

    // ` - 28` / ` - 6.5` / ` - 05v2` — the common single-episode form.
    if let Some(caps) =
        re(r"(?:^|[\s])-\s*(\d{1,3}(?:\.\d)?)\s*(?:v(\d+))?").captures(name)
    {
        release.episode = caps[1].parse().ok();
        release.version = caps.get(2).and_then(|m| m.as_str().parse().ok());
        return Some(caps.get(0).unwrap().start() + caps[0].find('-').unwrap_or(0));
    }

    // `EP28` / `Episode 28` / `E28`
    if let Some(caps) = re(r"(?i)\b(?:EPISODE|EP|E)\s*(\d{1,3}(?:\.\d)?)\b").captures(name) {
        release.episode = caps[1].parse().ok();
        return Some(caps.get(0).unwrap().start());
    }

    // `[28]` when the brackets carry only a number.
    if let Some(caps) = re(r"\s\[(\d{1,3}(?:\.\d)?)\]").captures(name) {
        release.episode = caps[1].parse().ok();
        return Some(caps.get(0).unwrap().start());
    }

    // Season only: `S01v2` / `Season 2` — usually a batch.
    if let Some(caps) = re(r"(?i)\bS(\d{1,2})\s*(?:v(\d+))?\b").captures(name) {
        release.season = caps[1].parse().ok();
        release.version = caps.get(2).and_then(|m| m.as_str().parse().ok());
        if release.version.is_some() {
            release.is_batch = true;
        }
        return None;
    }
    if let Some(caps) = re(r"(?i)\bSEASON\s*(\d{1,2})\b").captures(name) {
        release.season = caps[1].parse().ok();
    }
    None
}

pub fn extract_metadata(name: &str, release: &mut Release) {
    release.resolution = re(r"(?i)(480p|576p|720p|1080p|1440p|2160p|4k)\b")
        .captures(name)
        .map(|c| {
            let res = c[1].to_ascii_lowercase();
            if res == "4k" {
                "2160p".to_string()
            } else {
                res
            }
        });

    release.codec = if re(r"(?i)\b(hevc|x265|h\.?265)\b").is_match(name) {
        Some("hevc".to_string())
    } else if re(r"(?i)\b(av1)\b").is_match(name) {
        Some("av1".to_string())
    } else if re(r"(?i)\b(avc|x264|h\.?264)\b").is_match(name) {
        Some("avc".to_string())
    } else {
        None
    };

    release.source = if re(r"(?i)\bremux\b").is_match(name) {
        Some("remux".to_string())
    } else if re(r"(?i)\b(bd|bluray|blu-ray|bdremux|bd\d{3,4}p?)\b").is_match(name) {
        Some("bd".to_string())
    } else if re(r"(?i)\bweb-?dl\b").is_match(name) {
        Some("web-dl".to_string())
    } else if re(r"(?i)\bwebrip\b|\bweb\b").is_match(name) {
        Some("webrip".to_string())
    } else if re(r"(?i)\b(tv|tvrip)\b").is_match(name) {
        Some("tv".to_string())
    } else {
        None
    };

    release.crc = re(r"\[([0-9A-Fa-f]{8})\]")
        .captures(name)
        .map(|c| c[1].to_uppercase());

    if release.version.is_none() {
        release.version = re(r"(?i)(?:^|[\s\]])v(\d+)(?:[\s\]]|$)")
            .captures(name)
            .and_then(|c| c[1].parse().ok());
    }

    release.year = re(r"[(\[]((?:19|20)\d{2})[)\]]")
        .captures(name)
        .and_then(|c| c[1].parse().ok());

    if re(r"(?i)\b(batch|complete|compilation)\b").is_match(name) {
        release.is_batch = true;
    }
}

pub fn extract_languages(name: &str) -> Vec<String> {
    const LANGS: &[&str] = &[
        "english", "japanese", "chinese", "french", "spanish", "german", "portuguese",
        "russian", "italian", "korean", "hindi", "arabic", "thai", "vietnamese",
        "indonesian", "filipino", "polish", "turkish", "dutch", "swedish", "danish",
        "norwegian", "finnish", "czech", "romanian", "ukrainian", "multisub", "multi-sub",
    ];
    let mut found = Vec::new();
    for caps in re(r"\[([^\]]+)\]").captures_iter(name) {
        let inner = caps[1].to_ascii_lowercase();
        for lang in LANGS {
            if inner.contains(lang) {
                found.push(lang.to_string());
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

pub fn contains_dual_audio(name: &str) -> bool {
    re(r"(?i)\bdual[-\s]?audio\b|\bmulti[-\s]?audio\b").is_match(name)
}

pub fn is_metadata_bracket(inner: &str) -> bool {
    let lower = inner.to_ascii_lowercase();
    let bytes = inner.as_bytes();
    if inner.len() == 4 && bytes.iter().all(|b| b.is_ascii_digit()) && lower.starts_with('2') {
        return true;
    }
    re(r"(?i)^\s*(\d{3,4}p|4k|bd|bluray|web-?dl|webrip|hevc|x26[45]|avc|av1|aac|flac|dual[\s-]?audio|multisub|batch|complete)\s*$")
        .is_match(inner)
        || re(r"(?i)^[0-9A-F]{8}$").is_match(inner)
}

pub fn batch_range() -> &'static Regex {
    re(r"(?:^|[\s])\d{1,3}\s*-\s*\d{1,3}(?:\s|$)")
}
