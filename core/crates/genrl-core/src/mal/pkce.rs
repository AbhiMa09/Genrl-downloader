/// MAL's authorize page; the redirect is a loopback URL.
pub const AUTHORIZE_URL: &str = "https://myanimelist.net/v1/oauth2/authorize";
/// Real MyAnimeList OAuth token endpoint.
pub const TOKEN_URL: &str = "https://myanimelist.net/v1/oauth2/token";
/// Token endpoint path relative to the API base (for mocks and custom bases).
pub const TOKEN_PATH: &str = "/oauth2/token";
/// The current user's list.
pub const LIST_PATH: &str = "/users/@me/animelist";
/// `GET /anime/{id}?fields=` value that returns the links between seasons.
pub const RELATIONS_FIELD: &str = "related_anime";
/// Refresh this long before the access token expires (design: 5 minutes).
pub const REFRESH_MARGIN_SECS: i64 = 300;
/// State key holding the last successful sync, as a Unix timestamp.
pub const SYNC_STATE_KEY: &str = "last_mal_sync_at";

/// Fields asked for on the list endpoint: the raw status plus the node fields
/// the planner needs (`num_episodes` 0 means "unknown", so it maps to `None`).
pub const LIST_FIELDS: &str = "list_status{status,score,num_episodes_watched,updated_at},num_episodes,start_season,media_type,status,main_title,alternative_titles{synonyms,en,ja},main_picture,synopsis";
pub const PAGE_SIZE: usize = 1000;
pub const MAX_PAGES: usize = 20;

// ---------------------------------------------------------------------------
// OAuth: PKCE
// ---------------------------------------------------------------------------

/// A PKCE code verifier (RFC 7636 §4.1: 43–128 unreserved characters).
pub fn pkce_verifier() -> String {
    random_token(64)
}

/// `code_challenge` for the `plain` method (MAL only supports `plain`, per
/// their OAuth2 documentation): the plain challenge equals the verifier.
pub fn code_challenge(verifier: &str) -> String {
    verifier.to_string()
}

/// The authorize URL MAL expects: `response_type=code` with the S256 challenge
/// and the CSRF `state`, client id plainly (it is public by design).
pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str, challenge: &str) -> String {
    format!(
        "{AUTHORIZE_URL}?response_type=code&client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=plain",
        pct_encode(client_id),
        pct_encode(redirect_uri),
        pct_encode(state),
        pct_encode(challenge),
    )
}

/// Unreserved characters stay; everything else is percent-encoded. All the
/// values here are already URL-safe except the redirect URI, but encoding them
/// all is harmless and removes the doubt.
pub(crate) fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub(crate) fn pct_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(v) => {
                    out.push(v);
                    i += 3;
                }
                Err(_) => {
                    out.push(bytes[i]);
                    i += 1;
                }
            },
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Random characters from the PKCE alphabet (also used for the CSRF `state`).
pub(crate) fn random_token(len: usize) -> String {
    use rand::Rng;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect()
}

/// base64url, unpadded (URL-safe, no `=`). Only used to encode a SHA-256
/// digest, so 3-byte chunks always line up.
pub fn base64url(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n >> 0) as usize & 63] as char);
        }
    }
    out
}
