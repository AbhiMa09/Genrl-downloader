use std::path::Path;

use super::types::{MAX_NAME_LENGTH, VIDEO_EXTENSIONS};

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
