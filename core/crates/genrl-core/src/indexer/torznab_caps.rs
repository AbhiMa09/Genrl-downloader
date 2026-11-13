use quick_xml::events::Event;
use quick_xml::Reader;

use crate::db::Db;
use crate::indexer::redact::redact_secrets;
use crate::indexer::types::{Caps, IndexerError};

const SECONDS_24H: i64 = 86400;

/// Parse Torznab `t=caps` XML into a structured `Caps`.
///
/// Tolerant: If non-XML or empty, returns `IndexerError::Parse` with a redacted snippet (max 80 chars).
pub fn parse_caps(xml: &str) -> Result<Caps, IndexerError> {
    let trimmed = xml.trim();
    if trimmed.is_empty() {
        return Err(IndexerError::Parse("empty response body".into()));
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<!doctype html")
        || lower.starts_with("<html")
        || !trimmed.starts_with('<')
    {
        let snippet = redact_snippet(trimmed, 80);
        return Err(IndexerError::Parse(format!(
            "expected XML caps, found non-XML: {snippet}"
        )));
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut caps = Caps::default();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"error" => {
                        let mut code = 0;
                        let mut desc = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"code" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    code = s.parse().unwrap_or(0);
                                }
                            } else if attr.key.as_ref() == b"description" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    desc = s.to_string();
                                }
                            }
                        }
                        return Err(IndexerError::Remote {
                            code,
                            message: desc,
                        });
                    }
                    b"tv-search" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"available" {
                                caps.tv_available = attr.value.as_ref() == b"yes";
                            } else if attr.key.as_ref() == b"supportedParams" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    caps.tv_params = s
                                        .split(',')
                                        .map(|p| p.trim().to_ascii_lowercase())
                                        .filter(|p| !p.is_empty())
                                        .collect();
                                }
                            }
                        }
                    }
                    b"movie-search" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"available" {
                                caps.movie_available = attr.value.as_ref() == b"yes";
                            } else if attr.key.as_ref() == b"supportedParams" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    caps.movie_params = s
                                        .split(',')
                                        .map(|p| p.trim().to_ascii_lowercase())
                                        .filter(|p| !p.is_empty())
                                        .collect();
                                }
                            }
                        }
                    }
                    b"category" | b"subcat" => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"id" {
                                if let Ok(s) = std::str::from_utf8(&attr.value) {
                                    if let Ok(id) = s.parse::<u32>() {
                                        if !caps.categories.contains(&id) {
                                            caps.categories.push(id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                let snippet = redact_snippet(trimmed, 80);
                return Err(IndexerError::Parse(format!(
                    "XML error {e} near: {snippet}"
                )));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(caps)
}

/// Retrieve cached caps from SQLite if present and younger than 24 hours.
pub fn get_cached_caps(db: &Db, indexer_id: &str, now: i64) -> Result<Option<Caps>, IndexerError> {
    db.with(|c| {
        let mut stmt =
            c.prepare("SELECT caps_xml, fetched_at FROM indexer_caps WHERE indexer_id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![indexer_id])?;
        if let Some(row) = rows.next()? {
            let caps_xml: String = row.get(0)?;
            let fetched_at: i64 = row.get(1)?;
            if now - fetched_at < SECONDS_24H {
                return parse_caps(&caps_xml).map(Some).map_err(Into::into);
            }
        }
        Ok(None)
    })
    .map_err(|e| IndexerError::Parse(format!("db error reading caps: {e}")))
}

/// Persist raw caps XML in SQLite indexer_caps table.
pub fn save_cached_caps(
    db: &Db,
    indexer_id: &str,
    raw_xml: &str,
    now: i64,
) -> Result<(), IndexerError> {
    db.with(|c| {
        c.execute(
            "INSERT INTO indexer_caps (indexer_id, caps_xml, fetched_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(indexer_id) DO UPDATE SET
                 caps_xml = excluded.caps_xml,
                 fetched_at = excluded.fetched_at",
            rusqlite::params![indexer_id, raw_xml, now],
        )?;
        Ok(())
    })
    .map_err(|e| IndexerError::Parse(format!("db error saving caps: {e}")))
}

fn redact_snippet(text: &str, max_len: usize) -> String {
    let redacted = redact_secrets(text);
    if redacted.len() > max_len {
        format!("{}...", &redacted[..max_len])
    } else {
        redacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_caps_full() {
        let xml = include_str!("../../tests/fixtures/torznab/caps_full.xml");
        let caps = parse_caps(xml).expect("must parse full caps");
        assert!(caps.tv_available);
        assert!(caps.movie_available);
        assert!(caps.tv_params.contains(&"q".to_string()));
        assert!(caps.tv_params.contains(&"season".to_string()));
        assert!(caps.tv_params.contains(&"ep".to_string()));
        assert!(caps.tv_params.contains(&"imdbid".to_string()));
        assert!(caps.tv_params.contains(&"tvdbid".to_string()));
        assert!(caps.tv_params.contains(&"tmdbid".to_string()));
        assert!(caps.movie_params.contains(&"imdbid".to_string()));
        assert!(caps.movie_params.contains(&"tmdbid".to_string()));
        assert!(caps.categories.contains(&2000));
        assert!(caps.categories.contains(&2040));
        assert!(caps.categories.contains(&2045));
        assert!(caps.categories.contains(&5000));
        assert!(caps.categories.contains(&5040));
        assert!(caps.categories.contains(&5070));
    }

    #[test]
    fn test_parse_caps_minimal() {
        let xml = include_str!("../../tests/fixtures/torznab/caps_minimal.xml");
        let caps = parse_caps(xml).expect("must parse minimal caps");
        assert!(!caps.tv_available);
        assert!(!caps.movie_available);
        assert!(caps.categories.contains(&2000));
        assert!(caps.categories.contains(&5000));
    }

    #[test]
    fn test_parse_caps_html_login_redacts_snippet() {
        let html = "<!DOCTYPE html><html><body>Login with apikey=SECRET_12345678</body></html>";
        let err = parse_caps(html).expect_err("html must fail parsing");
        let msg = err.to_string();
        assert!(msg.contains("expected XML caps, found non-XML"));
        assert!(!msg.contains("SECRET_12345678"));
        assert!(msg.contains("apikey=[REDACTED]"));
    }

    #[test]
    fn test_parse_caps_empty_body() {
        let err = parse_caps("   ").expect_err("empty body must fail");
        assert!(err.to_string().contains("empty response body"));
    }

    #[test]
    fn test_cached_caps_24h_expiry() {
        let db = Db::open_in_memory().unwrap();
        let xml = include_str!("../../tests/fixtures/torznab/caps_full.xml");
        let now = 1_700_000_000;

        // Save caps
        save_cached_caps(&db, "test_idx", xml, now).expect("save caps");

        // Fresh within 24h
        let cached = get_cached_caps(&db, "test_idx", now + 3600)
            .expect("read cached")
            .expect("must be Some");
        assert!(cached.tv_available);

        // Expired after 24h
        let expired = get_cached_caps(&db, "test_idx", now + 86401).expect("read expired");
        assert!(expired.is_none(), "must return None after 24h");
    }
}
