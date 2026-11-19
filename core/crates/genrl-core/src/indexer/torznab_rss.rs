use quick_xml::events::Event;
use quick_xml::Reader;

use crate::indexer::redact::redact_secrets;
use crate::indexer::types::{DownloadSource, IndexerError, Release};

/// Parse Torznab search-result RSS XML into a list of `Release` records.
///
/// Tolerant: Handles non-XML input without panic (returning IndexerError::Parse),
/// tolerates missing item attributes (defaults 0/None), and extracts Remote errors.
pub fn parse_rss(indexer_id: &str, xml: &str) -> Result<Vec<Release>, IndexerError> {
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
            "expected XML RSS, found non-XML: {snippet}"
        )));
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut releases = Vec::new();
    let mut buf = Vec::new();

    // Track state inside an <item>
    let mut in_item = false;
    let mut current_tag = Vec::new();

    let mut item_title: Option<String> = None;
    let mut item_link: Option<String> = None;
    let mut item_pub_date: Option<String> = None;
    let mut item_size: Option<u64> = None;
    let mut item_seeders: Option<u32> = None;
    let mut item_leechers: Option<u32> = None;
    let mut item_peers: Option<u32> = None;
    let mut item_info_hash: Option<String> = None;
    let mut item_magnet: Option<String> = None;
    let mut item_enclosure_url: Option<String> = None;
    let mut item_enclosure_len: Option<u64> = None;
    let mut item_categories: Vec<u32> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                match name.as_ref() {
                    b"item" => {
                        in_item = true;
                        item_title = None;
                        item_link = None;
                        item_pub_date = None;
                        item_size = None;
                        item_seeders = None;
                        item_leechers = None;
                        item_peers = None;
                        item_info_hash = None;
                        item_magnet = None;
                        item_enclosure_url = None;
                        item_enclosure_len = None;
                        item_categories.clear();
                    }
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
                    _ => {
                        current_tag = name.as_ref().to_vec();
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
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
                    b"enclosure" if in_item => {
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"url" {
                                if let Ok(s) = attr.unescape_value() {
                                    item_enclosure_url = Some(s.to_string());
                                }
                            } else if attr.key.as_ref() == b"length" {
                                if let Ok(s) = attr.unescape_value() {
                                    item_enclosure_len = s.parse().ok();
                                }
                            }
                        }
                    }
                    b"torznab:attr" if in_item => {
                        let mut attr_name = String::new();
                        let mut attr_val = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"name" {
                                if let Ok(s) = attr.unescape_value() {
                                    attr_name = s.to_ascii_lowercase();
                                }
                            } else if attr.key.as_ref() == b"value" {
                                if let Ok(s) = attr.unescape_value() {
                                    attr_val = s.to_string();
                                }
                            }
                        }
                        match attr_name.as_str() {
                            "size" => item_size = attr_val.parse().ok(),
                            "seeders" => item_seeders = attr_val.parse().ok(),
                            "leechers" => item_leechers = attr_val.parse().ok(),
                            "peers" => item_peers = attr_val.parse().ok(),
                            "infohash" => item_info_hash = Some(attr_val.to_ascii_lowercase()),
                            "magneturl" => item_magnet = Some(attr_val),
                            "category" => {
                                if let Ok(cat_id) = attr_val.parse::<u32>() {
                                    if !item_categories.contains(&cat_id) {
                                        item_categories.push(cat_id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_item => {
                if let Ok(text) = e.unescape() {
                    let text = text.trim();
                    match current_tag.as_slice() {
                        b"title" => item_title = Some(text.to_string()),
                        b"link" => item_link = Some(text.to_string()),
                        b"pubDate" => item_pub_date = Some(text.to_string()),
                        b"size" => {
                            if item_size.is_none() {
                                item_size = text.parse().ok();
                            }
                        }
                        b"category" => {
                            if let Ok(id) = text.parse::<u32>() {
                                if !item_categories.contains(&id) {
                                    item_categories.push(id);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if e.name().as_ref() == b"item" {
                    in_item = false;
                    if let Some(title) = item_title.take() {
                        // Pick download source: magneturl preferred, else enclosure url, else link
                        let download = if let Some(mag) = item_magnet.take() {
                            Some(DownloadSource::Magnet(mag))
                        } else if let Some(enc) = item_enclosure_url.take() {
                            if enc.starts_with("magnet:") {
                                Some(DownloadSource::Magnet(enc))
                            } else {
                                Some(DownloadSource::TorrentUrl(enc))
                            }
                        } else if let Some(link) = item_link.take() {
                            if link.starts_with("magnet:") {
                                Some(DownloadSource::Magnet(link))
                            } else if link.starts_with("http://") || link.starts_with("https://") {
                                Some(DownloadSource::TorrentUrl(link))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        if let Some(download) = download {
                            let size_bytes = item_size.or(item_enclosure_len).unwrap_or(0);
                            let seeders = item_seeders.unwrap_or(0);
                            let leechers = item_leechers.unwrap_or_else(|| {
                                item_peers
                                    .map(|peers| peers.saturating_sub(seeders))
                                    .unwrap_or(0)
                            });

                            releases.push(Release {
                                title,
                                indexer_id: indexer_id.to_string(),
                                size_bytes,
                                seeders,
                                leechers,
                                download,
                                info_hash: item_info_hash.take(),
                                categories: item_categories.clone(),
                                native_category: None,
                                published_date: item_pub_date.take(),
                                trusted: false,
                                remake: false,
                            });
                        }
                    }
                }
                current_tag.clear();
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

    Ok(releases)
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
    fn test_parse_movie_xml() {
        let xml = include_str!("../../tests/fixtures/torznab/search_movie.xml");
        let items = parse_rss("prowlarr", xml).expect("must parse movies");
        assert_eq!(items.len(), 3);

        // Item 1: magneturl preferred
        assert_eq!(
            items[0].title,
            "Dune.Part.Two.2024.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX"
        );
        assert!(matches!(items[0].download, DownloadSource::Magnet(_)));
        assert_eq!(items[0].seeders, 45);
        assert_eq!(items[0].leechers, 5); // 50 peers - 45 seeders
        assert_eq!(items[0].size_bytes, 2500000000);
        assert_eq!(items[0].categories, vec![2000, 2040]);

        // Item 2: enclosure url only
        assert_eq!(
            items[1].title,
            "Oppenheimer.2023.2160p.UHD.BluRay.x265-TERMINAL"
        );
        assert!(matches!(items[1].download, DownloadSource::TorrentUrl(_)));
        assert_eq!(items[1].seeders, 20);
        assert_eq!(items[1].leechers, 5); // 25 peers - 20 seeders

        // Item 3: missing seeders defaults to 0
        assert_eq!(items[2].title, "Poor.Things.2023.1080p.WEBRip.x264-YTS");
        assert_eq!(items[2].seeders, 0);
        assert_eq!(items[2].leechers, 0);
    }

    #[test]
    fn test_parse_tv_xml_with_batch() {
        let xml = include_str!("../../tests/fixtures/torznab/search_tv.xml");
        let items = parse_rss("prowlarr", xml).expect("must parse tv");
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[1].title,
            "Shogun.2024.Season.1.Complete.1080p.WEB-DL.DDP5.1.Atmos.H.264-FLUX"
        );
        assert_eq!(items[1].seeders, 120);
        assert_eq!(items[1].leechers, 20); // 140 - 120
    }

    #[test]
    fn test_parse_error_xml() {
        let xml = include_str!("../../tests/fixtures/torznab/error.xml");
        let err = parse_rss("prowlarr", xml).expect_err("must parse error");
        match err {
            IndexerError::Remote { code, message } => {
                assert_eq!(code, 100);
                assert_eq!(message, "Invalid API Key");
            }
            other => panic!("expected Remote error, got: {other:?}"),
        }
    }

    #[test]
    fn test_parse_non_xml_html_redacted() {
        let html = "<!DOCTYPE html><html><body>Login with apikey=1234567890abcdef</body></html>";
        let err = parse_rss("prowlarr", html).expect_err("html must fail");
        let msg = err.to_string();
        assert!(msg.contains("expected XML RSS, found non-XML"));
        assert!(!msg.contains("1234567890abcdef"));
        assert!(msg.contains("apikey=[REDACTED]"));
    }

    #[test]
    fn test_missing_pubdate_and_size_defaults_cleanly() {
        let xml = r#"<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <item>
              <title>Minimal Item</title>
              <link>magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567</link>
            </item>
          </channel>
        </rss>"#;
        let items = parse_rss("prowlarr", xml).expect("must parse minimal item");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Minimal Item");
        assert_eq!(items[0].size_bytes, 0);
        assert_eq!(items[0].seeders, 0);
        assert_eq!(items[0].leechers, 0);
        assert_eq!(items[0].published_date, None);
    }
}
