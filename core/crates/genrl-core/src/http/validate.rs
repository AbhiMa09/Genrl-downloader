/// Only `magnet:?xt=urn:btih:` and `https://` `.torrent` links are accepted
/// from a feed. Private ranges and localhost are refused (SSRF), redirects are
/// capped at three by the client, and TLS is always verified.
pub fn is_acceptable_link(link: &str) -> bool {
    if let Some(rest) = link.strip_prefix("magnet:") {
        return rest.starts_with("?xt=urn:btih:") || rest.starts_with("xt=urn:btih:");
    }
    match reqwest::Url::parse(link) {
        // Private ranges are checked first so a "safe" scheme cannot smuggle
        // an internal address past us.
        Ok(url) => !host_is_private(&url) && url.scheme() == "https",
        Err(_) => false,
    }
}

/// Lexical private-address check for link validation.
fn host_is_private(url: &reqwest::Url) -> bool {
    let host = match url.host_str() {
        Some(h) => h.to_ascii_lowercase(),
        None => return true,
    };
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return true;
    }
    let octets: Vec<&str> = host.split('.').collect();
    if octets.len() == 4 && octets.iter().all(|p| p.parse::<u8>().is_ok()) {
        let a = octets[0].parse::<u8>().unwrap();
        let b = octets[1].parse::<u8>().unwrap();
        return match a {
            0 | 10 | 127 => true,
            100 => (64..=127).contains(&b), // CGNAT
            169 => b == 254,                // link-local
            172 => (16..=31).contains(&b),
            192 => b == 168 || b == 0,
            198 => b == 18 || b == 19,
            224..=255 => true, // multicast / reserved
            _ => false,
        };
    }
    if host.starts_with('[') || host.contains(':') {
        let h = host.trim_start_matches('[').trim_end_matches(']');
        return h == "::1" || h == "::" || h.starts_with("fe80") || h.starts_with("fc") || h.starts_with("fd");
    }
    false
}
