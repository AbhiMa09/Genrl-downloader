//! Nyaa search over RSS.
//!
//! One request per query through the shared gate (which owns the 1-per-2s
//! budget, the 10-minute cache and the mirror list). HTML pages are never
//! scraped: everything we need is in the feed, and magnets are built from the
//! info hash the feed already carries.

use crate::error::{Error, Result};
use crate::http::{Gate, GetOptions, Service};
use crate::settings::Settings;

/// Anime, English-translated — the usual source of fansub releases.
pub const DEFAULT_CATEGORY: &str = "1_2";

/// A single `<item>` from the feed.
#[derive(Debug, Clone, PartialEq)]
pub struct FeedItem {
    pub title: String,
    /// Direct `.torrent` download (never an HTML page).
    pub link: String,
    /// `https://nyaa.si/view/<id>`
    pub guid: String,
    /// Unix seconds from `<pubDate>`.
    pub published_at: i64,
    pub seeders: u32,
    pub leechers: u32,
    pub downloads: u32,
    pub info_hash: String,
    pub category_id: String,
    pub category: String,
    pub size_bytes: u64,
    pub size_text: String,
    pub trusted: bool,
    pub remake: bool,
}

impl FeedItem {
    /// A magnet built from the feed's own info hash — no extra request.
    pub fn magnet(&self) -> String {
        magnet_from(&self.info_hash, &self.title)
    }
}

/// What to ask Nyaa for.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    /// Nyaa category id, e.g. `1_2` (Anime English-translated), `1_4` (Raw).
    pub category: Option<String>,
    /// `0` all, `1` trusted only.
    pub filter: u8,
    /// Sort by seeders, descending: we want healthy releases first.
    pub sort_by_seeders: bool,
    /// Drop results below this many seeders after parsing.
    pub min_seeders: u32,
    /// Keep at most this many items.
    pub limit: usize,
}

impl SearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        SearchQuery {
            text: text.into(),
            category: Some(DEFAULT_CATEGORY.to_string()),
            filter: 0,
            sort_by_seeders: true,
            min_seeders: 0,
            limit: 50,
        }
    }

    pub fn without_category(mut self) -> Self {
        self.category = None;
        self
    }
}

/// Nyaa's RSS endpoint. Sorting by seeders keeps the healthiest releases first.
pub fn feed_url(base: &str, query: &SearchQuery) -> String {
    let mut url = format!(
        "{}/?page=rss&q={}&f={}",
        base.trim_end_matches('/'),
        encode(&query.text),
        query.filter
    );
    if let Some(category) = &query.category {
        url.push_str(&format!("&c={}", encode(category)));
    }
    if query.sort_by_seeders {
        url.push_str("&s=seeders&o=desc");
    }
    url
}

/// Search Nyaa. Falls back to the configured mirrors when the primary fails.
pub async fn search(
    gate: &Gate,
    settings: &Settings,
    query: &SearchQuery,
) -> Result<Vec<FeedItem>> {
    let mut bases = vec![settings.nyaa_base_url.clone()];
    bases.extend(settings.nyaa_mirrors.iter().cloned());

    let mut last_err: Option<Error> = None;
    for base in &bases {
        let url = feed_url(base, query);
        // The cache key is the full URL, so each base caches under its own key.
        let opts = GetOptions::default();
        // Any failure — unreachable, error status or a malformed feed — moves
        // on to the next mirror; the last one decides the error the caller sees.
        let outcome = match gate.get_with(Service::Nyaa, &url, &opts).await {
            Ok(fetched) => fetched
                .text()
                .and_then(|text| parse_feed(&text))
                .map(|mut items| {
                    items.retain(|i| i.seeders >= query.min_seeders);
                    items.sort_by_key(|a| std::cmp::Reverse(a.seeders));
                    if items.len() > query.limit {
                        items.truncate(query.limit);
                    }
                    items
                }),
            Err(err) => Err(err),
        };
        match outcome {
            Ok(items) => return Ok(items),
            Err(err) => {
                tracing::warn!(%base, error = %err, "nyaa base failed, trying the next mirror");
                last_err = Some(err);
            }
        }
    }
    Err(last_err.unwrap_or_else(|| Error::Other("no Nyaa base URL configured".into())))
}

/// Parse an RSS document into feed items.
///
/// Text comes from the elements we know about only: `<description>` holds HTML
/// and is never parsed or rendered. Anything malformed is rejected outright
/// rather than guessed at, and the item count is capped so a hostile feed
/// cannot exhaust memory.
pub fn parse_feed(xml: &str) -> Result<Vec<FeedItem>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    const MAX_ITEMS: usize = 1000;

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items: Vec<FeedItem> = Vec::new();
    let mut current: Option<FeedItem> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let name = e.name();
                let raw = name.as_ref();
                if raw == b"item" {
                    if items.len() >= MAX_ITEMS {
                        break;
                    }
                    current = Some(FeedItem {
                        published_at: 0,
                        seeders: 0,
                        leechers: 0,
                        downloads: 0,
                        info_hash: String::new(),
                        category_id: String::new(),
                        category: String::new(),
                        size_bytes: 0,
                        size_text: String::new(),
                        trusted: false,
                        remake: false,
                        title: String::new(),
                        link: String::new(),
                        guid: String::new(),
                    });
                    continue;
                }
                if let Some(item) = current.as_mut() {
                    let text = match raw {
                        b"title" => Some(read_text(&mut reader, name)?),
                        b"link" => Some(read_text(&mut reader, name)?),
                        b"guid" => Some(read_text(&mut reader, name)?),
                        b"pubDate" => Some(read_text(&mut reader, name)?),
                        b"nyaa:seeders" | b"seeders" => Some(read_text(&mut reader, name)?),
                        b"nyaa:leechers" | b"leechers" => Some(read_text(&mut reader, name)?),
                        b"nyaa:downloads" | b"downloads" => Some(read_text(&mut reader, name)?),
                        b"nyaa:infoHash" | b"infoHash" => Some(read_text(&mut reader, name)?),
                        b"nyaa:categoryId" | b"categoryId" => Some(read_text(&mut reader, name)?),
                        b"nyaa:category" | b"category" => Some(read_text(&mut reader, name)?),
                        b"nyaa:size" | b"size" => Some(read_text(&mut reader, name)?),
                        b"nyaa:trusted" | b"trusted" => Some(read_text(&mut reader, name)?),
                        b"nyaa:remake" | b"remake" => Some(read_text(&mut reader, name)?),
                        _ => None,
                    };
                    if let Some(text) = text {
                        match raw {
                            b"title" => item.title = text,
                            b"link" => item.link = text,
                            b"guid" => item.guid = text,
                            b"pubDate" => item.published_at = parse_pub_date(&text).unwrap_or(0),
                            b"nyaa:seeders" | b"seeders" => {
                                item.seeders = text.parse().unwrap_or(0)
                            }
                            b"nyaa:leechers" | b"leechers" => {
                                item.leechers = text.parse().unwrap_or(0)
                            }
                            b"nyaa:downloads" | b"downloads" => {
                                item.downloads = text.parse().unwrap_or(0)
                            }
                            b"nyaa:infoHash" | b"infoHash" => item.info_hash = text.to_lowercase(),
                            b"nyaa:categoryId" | b"categoryId" => item.category_id = text,
                            b"nyaa:category" | b"category" => item.category = text,
                            b"nyaa:size" | b"size" => {
                                item.size_bytes = parse_size(&text);
                                item.size_text = text;
                            }
                            b"nyaa:trusted" | b"trusted" => item.trusted = is_yes(&text),
                            b"nyaa:remake" | b"remake" => item.remake = is_yes(&text),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                if e.name().as_ref() == b"item" {
                    if let Some(item) = current.take() {
                        if !item.title.is_empty() && !item.link.is_empty() {
                            items.push(item);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(Error::Other(format!("malformed RSS feed: {err}")));
            }
        }
    }

    Ok(items)
}

fn read_text(
    reader: &mut quick_xml::Reader<&[u8]>,
    name: quick_xml::name::QName<'_>,
) -> Result<String> {
    let text = reader
        .read_text(name)
        .map_err(|err| Error::Other(format!("malformed RSS feed: {err}")))?;
    Ok(text.trim().to_string())
}

fn is_yes(value: &str) -> bool {
    value.trim().eq_ignore_ascii_case("yes")
}

/// Build a magnet link from an info hash. Only `xt=urn:btih:` is ever produced.
pub fn magnet_from(info_hash: &str, title: &str) -> String {
    format!(
        "magnet:?xt=urn:btih:{}&dn={}",
        info_hash.trim(),
        encode(title)
    )
}

/// Parse Nyaa's human sizes: `6.6 GiB`, `700.5 MiB`, `12345 B`.
pub fn parse_size(text: &str) -> u64 {
    let text = text.trim();
    let mut parts = text.split_whitespace();
    let number: f64 = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0.0);
    let unit = parts.next().unwrap_or("");
    let multiplier = match unit.to_ascii_lowercase().as_str() {
        "kib" | "kb" => 1024f64,
        "mib" | "mb" => 1024f64 * 1024.0,
        "gib" | "gb" => 1024f64 * 1024.0 * 1024.0,
        "tib" | "tb" => 1024f64 * 1024.0 * 1024.0 * 1024.0,
        _ => 1.0,
    };
    (number * multiplier) as u64
}

/// Parse an RFC-822 `<pubDate>` such as `Sat, 12 Sep 2026 15:45:00 -0000`
/// into Unix seconds.
pub fn parse_pub_date(text: &str) -> Option<i64> {
    let fields: Vec<&str> = text.split_whitespace().collect();
    // "Sat, 12 Sep 2026 15:45:00 -0000" — weekday is optional.
    let start = if fields.first().map(|f| f.ends_with(',')).unwrap_or(false) {
        1
    } else {
        0
    };
    let day: i64 = fields.get(start)?.parse().ok()?;
    let month = month_index(fields.get(start + 1)?)?;
    let year: i64 = fields.get(start + 2)?.parse().ok()?;
    let time = fields.get(start + 3)?;
    let mut clock = time.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next().unwrap_or("0").parse().ok()?;
    let second: i64 = clock.next().unwrap_or("0").parse().ok()?;
    let offset = parse_offset(fields.get(start + 4));

    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

fn month_index(name: &str) -> Option<i64> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let lower = name.to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|m| lower.starts_with(m))
        .map(|i| i as i64 + 1)
}

fn parse_offset(field: Option<&&str>) -> i64 {
    let Some(raw) = field else { return 0 };
    let raw = raw.trim();
    let sign = match raw.as_bytes().first() {
        Some(b'-') => -1,
        _ => 1,
    };
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() < 4 {
        return 0;
    }
    let hours: i64 = digits[..2].parse().unwrap_or(0);
    let minutes: i64 = digits[2..4].parse().unwrap_or(0);
    sign * (hours * 3600 + minutes * 60)
}

/// Howard Hinnant's days-from-civil: proleptic Gregorian date to days since epoch.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = include_str!("../tests/fixtures/nyaa_frieren.xml");

    #[test]
    fn parses_a_real_feed() {
        let items = parse_feed(SAMPLE).expect("real feed parses");
        assert!(items.len() > 50, "expected many items, got {}", items.len());

        let first = &items[0];
        assert!(first.title.contains("Frieren"), "{}", first.title);
        assert_eq!(first.link, "https://nyaa.si/download/2160092.torrent");
        assert_eq!(first.guid, "https://nyaa.si/view/2160092");
        assert_eq!(first.seeders, 53);
        assert_eq!(first.leechers, 2);
        assert_eq!(first.info_hash, "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b");
        assert_eq!(first.category_id, "1_2");
        assert!(
            first.size_bytes > 6 * 1024 * 1024 * 1024,
            "{}",
            first.size_bytes
        );
        assert!(!first.trusted);
        assert!(first.published_at > 1_700_000_000, "pubDate parsed");
    }

    #[test]
    fn every_item_has_the_basics() {
        let items = parse_feed(SAMPLE).unwrap();
        for item in &items {
            assert!(!item.title.is_empty());
            assert!(
                item.link.starts_with("https://nyaa.si/download/"),
                "{}",
                item.link
            );
            assert_eq!(item.info_hash.len(), 40, "hash for {}", item.title);
            assert!(item.size_bytes > 0, "size for {}", item.title);
        }
    }

    #[test]
    fn magnets_only_carry_bittorrent_hashes() {
        let m = magnet_from(
            "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b",
            "Sousou no Frieren - 01",
        );
        assert!(m.starts_with("magnet:?xt=urn:btih:"));
        assert!(m.contains("dn=Sousou+no+Frieren"));
        assert!(crate::http::is_acceptable_link(&m));
    }

    #[test]
    fn feed_url_builds_a_seeder_sorted_query() {
        let q = SearchQuery::new("sousou no frieren 1080p");
        let url = feed_url("https://nyaa.si", &q);
        assert_eq!(
            url,
            "https://nyaa.si/?page=rss&q=sousou+no+frieren+1080p&f=0&c=1_2&s=seeders&o=desc"
        );
        assert_eq!(
            feed_url("https://nyaa.si/", &q),
            url,
            "trailing slash tolerated"
        );
    }

    #[test]
    fn sizes_and_dates_parse() {
        assert_eq!(parse_size("6.6 GiB"), 7_086_696_038);
        assert_eq!(parse_size("700.5 MiB"), 734_527_488);
        assert_eq!(parse_size("12345 B"), 12_345);

        let ts = parse_pub_date("Sat, 12 Sep 2026 15:45:00 -0000").unwrap();
        assert_eq!(ts, 1_789_227_900, "known timestamp");

        // Same shape but with a real +0200 offset: two hours earlier in UTC.
        let shifted = parse_pub_date("Mon, 31 Aug 2026 10:54:50 +0200").unwrap();
        assert_eq!(shifted, 1_788_166_490, "offset applied");
    }

    #[test]
    fn malformed_feed_is_rejected_not_guessed() {
        assert!(parse_feed("<rss><channel><item></rss>").is_err());
        assert!(parse_feed("").is_ok(), "empty feed is just empty");
    }

    #[test]
    fn a_hostile_feed_cannot_run_away() {
        let mut xml = String::from("<rss><channel>");
        for i in 0..5000 {
            xml.push_str(&format!(
                "<item><title>t{i}</title><link>https://nyaa.si/download/{i}.torrent</link></item>"
            ));
        }
        xml.push_str("</channel></rss>");
        let items = parse_feed(&xml).unwrap();
        assert!(
            items.len() <= 1000,
            "item count capped, got {}",
            items.len()
        );
    }
}
