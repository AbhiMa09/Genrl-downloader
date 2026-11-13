use regex::Regex;
use std::sync::LazyLock;

static SECRET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(api_?key|passkey|torrent_pass)=([^&\s#]+)").expect("valid secret regex")
});

/// Redacts secrets (apikey=, api_key=, passkey=, torrent_pass=) from strings or URLs.
pub fn redact_secrets(input: &str) -> String {
    SECRET_REGEX.replace_all(input, "$1=[REDACTED]").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_in_middle_of_url() {
        let url = "https://prowlarr.local/api/v1/torznab/3/api?apikey=SUPERSECRETKEY123&t=caps";
        let expected = "https://prowlarr.local/api/v1/torznab/3/api?apikey=[REDACTED]&t=caps";
        assert_eq!(redact_secrets(url), expected);
    }

    #[test]
    fn key_at_end_of_url() {
        let url = "https://prowlarr.local/api/v1/torznab/3/api?t=tvsearch&api_key=ENDSECRET999";
        let expected = "https://prowlarr.local/api/v1/torznab/3/api?t=tvsearch&api_key=[REDACTED]";
        assert_eq!(redact_secrets(url), expected);
    }

    #[test]
    fn several_secrets_in_one_string() {
        let input = "download from https://tracker.org/dl?passkey=MYPASSKEY&torrent_pass=TORPASS123 or api?apikey=KEY999";
        let expected = "download from https://tracker.org/dl?passkey=[REDACTED]&torrent_pass=[REDACTED] or api?apikey=[REDACTED]";
        assert_eq!(redact_secrets(input), expected);
    }

    #[test]
    fn no_secret_present_unchanged() {
        let url = "https://prowlarr.local/api/v1/torznab/3/api?t=caps&cat=2000,5000";
        assert_eq!(redact_secrets(url), url);
    }
}
