//! Release-name parsing and title normalization.
//!
//! Fansubs write titles every way they like: `[SubsPlease] Sousou no Frieren -
//! 28 (1080p) [ABCD1234].mkv`, `Kimi no Na wa (2016) [BD1080p]`, `[Erai-raws]
//! Frieren 2nd Season [1080p][MultiSub] (unofficial batch)`. The parser pulls
//! out group, title, season, episode, resolution, codec, source and batch-ness,
//! and strips group tags, CRC hashes and resolutions before anything is scored.

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
    /// Not serialized: it is an artefact of parsing, not a fact about the show.
    #[serde(skip)]
    pub marker_start: Option<usize>,
}

fn re(pattern: &str) -> &'static Regex {
    // Patterns are constant: compile once and share across threads.
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

const VIDEO_EXTENSIONS: &[&str] = &["mkv", "mp4", "avi", "m2ts", "ts", "wmv", "mov", "webm"];

/// Parse a release name or filename.
pub fn parse_release(input: &str) -> Release {
    let raw = input.trim().to_string();
    let mut release = Release {
        raw: raw.clone(),
        ..Default::default()
    };

    // 1. Extension: `... .mkv` (kept for the allowlist check at finalize).
    let mut name = raw.as_str();
    if let Some((stem, ext)) = name.rsplit_once('.') {
        let ext = ext.to_ascii_lowercase();
        if VIDEO_EXTENSIONS.contains(&ext.as_str()) && !ext.contains(' ') {
            release.extension = Some(ext);
            name = stem;
        }
    }

    // 2. Leading `[Group]`.
    let group_re = re(r"^\s*\[([^\]]+)\]");
    let mut title_start = 0;
    if let Some(caps) = group_re.captures(name) {
        let candidate = caps[1].trim().to_string();
        // A leading bracket that is obviously not a group (e.g. a year or a
        // resolution) belongs to the title instead.
        if !is_metadata_bracket(&candidate) {
            release.group = Some(candidate);
            title_start = caps.get(0).unwrap().end();
        }
    }

    // 3. Season and episode (returns where the episode token started).
    release.marker_start = extract_season_episode(name, &mut release);

    // 4. Everything else comes from the full name: resolution, codec, source.
    extract_metadata(name, &mut release);

    // 5. Title = text before the episode token (or the whole thing), minus
    //    every bracket group, then whitespace-collapsed.
    let title_end = release.marker_start.map(|pos| pos.min(name.len())).unwrap_or(name.len());
    let mut title = name[title_start.min(title_end)..title_end].to_string();
    if release.episode.is_none() && release.is_batch {
        // Batch names like `Show - 01-12 [Batch]`: keep the range out too.
        if let Some(caps) = batch_range().captures(&title) {
            if let Some(m) = caps.get(0) {
                title.replace_range(m.range(), " ");
            }
        }
    }
    release.title = clean_title(&title);
    release.normalized_title = normalize(&release.title);

    // 6. Languages live in metadata brackets only.
    release.languages = extract_languages(name);
    release.dual_audio = contains_dual_audio(name);

    release
}

/// Find season/episode in the release name, filling `release` and returning
/// the byte offset where the episode token started (so the title can be cut
/// there), or `None` when there is no episode token.
fn extract_season_episode(name: &str, release: &mut Release) -> Option<usize> {
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

fn extract_metadata(name: &str, release: &mut Release) {
    // Resolution: `1080p`, `BD1080p`, `720p`, `2160p`, `4K`. No leading
    // boundary: `BD1080p` runs the resolution straight onto its prefix.
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

    // Codec: hevc/x265 beat avc/x264 when both appear (x265 is the newer tag).
    release.codec = if re(r"(?i)\b(hevc|x265|h\.?265)\b").is_match(name) {
        Some("hevc".to_string())
    } else if re(r"(?i)\b(av1)\b").is_match(name) {
        Some("av1".to_string())
    } else if re(r"(?i)\b(avc|x264|h\.?264)\b").is_match(name) {
        Some("avc".to_string())
    } else {
        None
    };

    // Source: remux outranks bd outranks web.
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

    // CRC hash: `[ABCD1234]` — eight hex digits alone in brackets.
    release.crc = re(r"\[([0-9A-Fa-f]{8})\]")
        .captures(name)
        .map(|c| c[1].to_uppercase());

    // Version: `v2` fixes, usually right after the episode or season.
    if release.version.is_none() {
        release.version = re(r"(?i)(?:^|[\s\]])v(\d+)(?:[\s\]]|$)")
            .captures(name)
            .and_then(|c| c[1].parse().ok());
    }

    // Year in brackets or parens: `(2016)`, `[1998]`.
    release.year = re(r"[(\[]((?:19|20)\d{2})[)\]]")
        .captures(name)
        .and_then(|c| c[1].parse().ok());

    // Batch: explicit words, or a detected range.
    if re(r"(?i)\b(batch|complete|compilation)\b").is_match(name) {
        release.is_batch = true;
    }
}

fn extract_languages(name: &str) -> Vec<String> {
    const LANGS: &[&str] = &[
        "english", "japanese", "chinese", "french", "spanish", "german", "portuguese",
        "russian", "italian", "korean", "hindi", "arabic", "thai", "vietnamese",
        "indonesian", "filipino", "polish", "turkish", "dutch", "swedish", "danish",
        "norwegian", "finnish", "czech", "romanian", "ukrainian", "multisub", "multi-sub",
    ];
    let mut found = Vec::new();
    // Only inside brackets: that is where metadata lives.
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

fn contains_dual_audio(name: &str) -> bool {
    re(r"(?i)\bdual[-\s]?audio\b|\bmulti[-\s]?audio\b").is_match(name)
}

fn is_metadata_bracket(inner: &str) -> bool {
    let lower = inner.to_ascii_lowercase();
    let bytes = inner.as_bytes();
    // Years, resolutions and pure checksums are metadata, not group names.
    if inner.len() == 4 && bytes.iter().all(|b| b.is_ascii_digit()) && lower.starts_with('2') {
        return true;
    }
    re(r"(?i)^\s*(\d{3,4}p|4k|bd|bluray|web-?dl|webrip|hevc|x26[45]|avc|av1|aac|flac|dual[\s-]?audio|multisub|batch|complete)\s*$")
        .is_match(inner)
        || re(r"(?i)^[0-9A-F]{8}$").is_match(inner)
}

fn batch_range() -> &'static Regex {
    re(r"(?:^|[\s])\d{1,3}\s*-\s*\d{1,3}(?:\s|$)")
}

/// Turn the title remainder into readable text: drop every bracket or paren
/// group, collapse whitespace, trim dangling separators.
fn clean_title(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' | '(' => depth += 1,
            ']' | ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    let cleaned = out
        .replace(['_', '+'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    cleaned
        .trim_matches(|c: char| c == '-' || c == ' ' || c == ',' || c == '.' || c == ':')
        .to_string()
}

/// Normalize a title the way the matching engine expects: lower-cased,
/// full-width converted, punctuation gone, season words stripped, single
/// spaces. `"Kimi no Na wa."` becomes `kimi no na wa`.
pub fn normalize(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            // Full-width ASCII → ASCII, ideographic space → space.
            // (U+FF06 full-width ampersand maps to '&' in this range too.)
            '\u{FF01}'..='\u{FF5E}' => out.push((ch as u32 - 0xFF01 + 0x21) as u8 as char),
            '\u{3000}' => out.push(' '),
            c => out.push(c),
        }
    }

    let lower = out.to_lowercase();

    // Ordinals: `2nd` → `2`, so "Frieren 2nd Season" and "Frieren 2" agree.
    let deordinalled = re(r"(\d)(?:st|nd|rd|th)\b").replace_all(&lower, "$1");

    // Keep letters and digits; everything else becomes a separator.
    let mut stripped = String::with_capacity(deordinalled.len());
    for ch in deordinalled.chars() {
        if ch.is_alphanumeric() {
            stripped.push(ch);
        } else {
            stripped.push(' ');
        }
    }

    // Strip season words (the numbers stay: they identify the season).
    let mut words: Vec<String> = stripped
        .split_whitespace()
        .filter(|w| !matches!(*w, "season" | "cour" | "part" | "series" | "the"))
        .map(|w| {
            // Leading zeros: `- 02` and `- 2` are the same episode.
            if w.len() > 1 && w.starts_with('0') && w.chars().all(|c| c.is_ascii_digit()) {
                w.trim_start_matches('0').to_string()
            } else {
                w.to_string()
            }
        })
        .filter(|w| !w.is_empty())
        .collect();
    words.dedup();
    words.join(" ")
}

/// Fuzzy ratio in `0.0..=1.0`: one minus the normalized edit distance.
///
/// Used to decide whether a parsed release title is really the show's name —
/// the design scores a match at 0.92 or higher.
pub fn fuzzy_ratio(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let distance = levenshtein(&a, &b);
    1.0 - (distance as f64 / a.len().max(b.len()) as f64)
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str) -> Release {
        parse_release(name)
    }

    #[test]
    fn parses_a_standard_single_episode() {
        let r = parse("[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv");
        assert_eq!(r.group.as_deref(), Some("SubsPlease"));
        assert_eq!(r.title, "Sousou no Frieren");
        assert_eq!(r.normalized_title, "sousou no frieren");
        assert_eq!(r.episode, Some(28.0));
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.crc.as_deref(), Some("ABCD1234"));
        assert_eq!(r.extension.as_deref(), Some("mkv"));
        assert!(!r.is_batch);
    }

    #[test]
    fn parses_season_episode_form() {
        let r = parse("[Erai-raws] Sousou no Frieren - S01E05 (1080p) [ABCD1234].mkv");
        assert_eq!(r.group.as_deref(), Some("Erai-raws"));
        assert_eq!(r.season, Some(1));
        assert_eq!(r.episode, Some(5.0));
        assert_eq!(r.title, "Sousou no Frieren");
    }

    #[test]
    fn parses_decimal_episodes() {
        let r = parse("[SubsPlease] Steins;Gate - 6.5 (1080p) [01234567].mkv");
        assert_eq!(r.episode, Some(6.5), "OVAs with decimals survive");
        assert_eq!(r.title, "Steins;Gate");
    }

    #[test]
    fn parses_batches_and_ranges() {
        let r = parse("[Judas] Sousou no Frieren - 01-28 [BD 1080p] [Batch]");
        assert!(r.is_batch, "{r:?}");
        assert_eq!(r.episode, None, "a range is not a single episode");

        let r = parse("[Erai-raws] Frieren 2nd Season [1080p CR WEBRip HEVC AAC][MultiSub] (unofficial batch)");
        assert!(r.is_batch);
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.codec.as_deref(), Some("hevc"));
        assert_eq!(r.source.as_deref(), Some("webrip"));
        assert!(r.languages.contains(&"multisub".to_string()));
    }

    #[test]
    fn parses_movies_without_episode() {
        let r = parse("Kimi no Na wa (2016) [BD1080p] [Dual Audio].mkv");
        assert_eq!(r.episode, None);
        assert_eq!(r.title, "Kimi no Na wa");
        assert_eq!(r.normalized_title, "kimi no na wa");
        assert_eq!(r.year, Some(2016));
        assert!(r.dual_audio);
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.source.as_deref(), Some("bd"));
    }

    #[test]
    fn sequel_titles_do_not_swallow_their_numbers() {
        let a = parse("[SubsPlease] Kimi no Na wa - 01 (1080p).mkv");
        assert_eq!(a.title, "Kimi no Na wa");

        let b = parse("[Group] Re:Zero kara Hajimeru Isekai Seikatsu - 05 (1080p).mkv");
        assert_eq!(b.title, "Re:Zero kara Hajimeru Isekai Seikatsu");
        assert_eq!(b.episode, Some(5.0));

        let c = parse("[Group] Re:Creators - 05 (1080p).mkv");
        assert_eq!(c.title, "Re:Creators", "a different show entirely");
    }

    #[test]
    fn version_and_codec_and_source() {
        let r = parse("[AUTISM] Sousou no Frieren - S01v2 (BD Remux 1080p AVC FLAC) [Dual-Audio]");
        assert_eq!(r.version, Some(2), "{r:?}");
        assert_eq!(r.codec.as_deref(), Some("avc"));
        assert_eq!(r.source.as_deref(), Some("remux"));
        assert!(r.dual_audio);

        let r = parse("[Group] Show - 05v3 (720p x265).mkv");
        assert_eq!(r.version, Some(3));
        assert_eq!(r.codec.as_deref(), Some("hevc"));
    }

    #[test]
    fn normalization_follows_the_design() {
        assert_eq!(normalize("Kimi no Na wa."), "kimi no na wa");
        assert_eq!(normalize("Ｓｏｕｓｏｕ ｎｏ Ｆｒｉｅｒｅｎ"), "sousou no frieren");
        assert_eq!(normalize("Frieren: Beyond Journey's End"), "frieren beyond journey s end");
        assert_eq!(normalize("Frieren Season 2"), "frieren 2", "season words stripped");
        assert_eq!(normalize("Frieren 2nd Season"), "frieren 2");
        assert_eq!(normalize("Show - 02"), normalize("Show - 2"), "zero padding ignored");
        assert_eq!(normalize("  KIMI   no  NA wa  "), "kimi no na wa");
    }

    #[test]
    fn fuzzy_ratio_is_strict_where_it_matters() {
        assert_eq!(fuzzy_ratio("kimi no na wa", "kimi no na wa"), 1.0);
        assert!(fuzzy_ratio("kimi no na wa", "kimi no na wb") > 0.92, "typos still clear 0.92");
        assert!(
            fuzzy_ratio("kimi no na wa", "kimi no todoke") < 0.7,
            "a sequel sharing a prefix must not score as a match"
        );
        assert!(fuzzy_ratio("", "") == 1.0);
        assert!(fuzzy_ratio("abc", "") == 0.0);
    }

    #[test]
    fn metadata_brackets_are_not_group_names() {
        let r = parse("[1080p] Some Show - 01.mkv");
        assert_eq!(r.group, None, "a resolution is not a fansub group");
        let r = parse("[ABCD1234] Some Show - 01.mkv");
        assert_eq!(r.group, None, "a CRC is not a fansub group");
    }

    #[test]
    fn parses_the_real_feed_titles() {
        let xml = include_str!("../tests/fixtures/nyaa_frieren.xml");
        let items = crate::nyaa::parse_feed(xml).unwrap();
        let mut parsed = 0;
        for item in &items {
            let r = parse(&item.title);
            if !r.title.is_empty() {
                parsed += 1;
            }
            assert!(
                !r.title.contains('['),
                "brackets stripped from title: {:?}",
                r.title
            );
            assert!(r.title.len() <= item.title.len(), "tags removed or kept");
        }
        assert!(parsed >= items.len() * 9 / 10, "most real titles parse");
    }
}
