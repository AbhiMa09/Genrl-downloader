use regex::Regex;
use std::sync::OnceLock;

fn re(pattern: &str) -> &'static Regex {
    use std::collections::HashMap;
    use std::sync::Mutex;
    static CACHE: OnceLock<Mutex<HashMap<String, &'static Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(rx) = map.get(pattern) {
        return rx;
    }
    let rx = Box::leak(Box::new(
        Regex::new(pattern).expect("static regex is valid"),
    ));
    map.insert(pattern.to_string(), rx);
    rx
}

/// Turn the title remainder into readable text: drop every bracket or paren
/// group, collapse whitespace, trim dangling separators.
pub fn clean_title(text: &str) -> String {
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
            '\u{FF01}'..='\u{FF5E}' => out.push((ch as u32 - 0xFF01 + 0x21) as u8 as char),
            '\u{3000}' => out.push(' '),
            c => out.push(c),
        }
    }

    let lower = out.to_lowercase();
    let deordinalled = re(r"(\d)(?:st|nd|rd|th)\b").replace_all(&lower, "$1");

    let mut stripped = String::with_capacity(deordinalled.len());
    for ch in deordinalled.chars() {
        if ch.is_alphanumeric() {
            stripped.push(ch);
        } else {
            stripped.push(' ');
        }
    }

    let mut words: Vec<String> = stripped
        .split_whitespace()
        .filter(|w| !matches!(*w, "season" | "cour" | "part" | "series" | "the"))
        .map(|w| {
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
