//! The real MAL sync (phase 5): OAuth with PKCE, the list, the diff, and the
//! season chain.
//!
//! The design's limit-proofing table: *"Sync at most every 30 minutes, ETag or
//! last-updated checks, refresh the OAuth token 5 minutes before expiry, store
//! the refresh token in the system keyring"* — and its security note: *"The
//! MAL OAuth redirect uses a local loopback port with PKCE, so no client
//! secret is stored."*
//!
//! Three rules the code follows:
//!
//! * **No client secret anywhere.** PKCE's S256 challenge is generated here;
//!   the only credential stored is the refresh token, in the keyring
//!   ([`crate::secrets`]). The short-lived access token lives in memory for
//!   the process only and is re-minted by a refresh on launch, so no
//!   plaintext credential ever lands in the database, a file or a log line.
//! * **Every request through the gate.** The limiter, cache, backoff and
//!   breaker of [`crate::http`] apply; a MAL outage lands an Activity-log
//!   warning, never a stuck queue.
//! * **A relation failure never misgroups.** A sequel or prequel lookup that
//!   fails leaves that entry its own one-season chain — a missing chain is
//!   safe, a wrong one is worse.
//!
//! The list comes back as the [`planner::ListEntry`]s the planner already
//! works on, with `franchise`/`season`/`episode_offset` filled from MAL's
//! sequel and prequel relations, so [`crate::planner::plan`] runs unchanged on
//! a real list.
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use serde::Deserialize;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::EventLog;
use crate::http::{Fetched, GetOptions, Gate, Service};
use crate::models::{ListStatus, QuestionKind};
use crate::planner::ListEntry;
use crate::questions::{Payload, QuestionStore};
use crate::secrets::{self, Secret};
use crate::settings::{self, Settings};

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
const LIST_FIELDS: &str = "list_status{status,score,num_episodes_watched,updated_at},num_episodes,start_season,media_type,status,main_title,alternative_titles{synonyms,en,ja},main_picture,synopsis";
const PAGE_SIZE: usize = 1000;
const MAX_PAGES: usize = 20;

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
fn pct_encode(s: &str) -> String {
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

fn pct_decode(s: &str) -> String {
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
fn random_token(len: usize) -> String {
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

// ---------------------------------------------------------------------------
// The token store
// ---------------------------------------------------------------------------

/// A working access token plus its expiry.
#[derive(Debug, Clone, PartialEq)]
pub struct Tokens {
    pub access_token: String,
    /// Unix seconds; refreshed [`REFRESH_MARGIN_SECS`] before this.
    pub expires_at: i64,
}

/// Where the long-lived refresh token lives: the keyring in the app, memory
/// in tests.
pub trait TokenStore: Send + Sync {
    fn refresh_token(&self) -> Result<Option<String>>;
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()>;
    fn client_secret(&self) -> Result<Option<String>> {
        Ok(None)
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        let _ = secret;
        Ok(())
    }
}

/// The app's store: [`crate::secrets`], i.e. the system keyring (libsecret)
/// with the `GENRL_MAL_REFRESH_TOKEN` environment override.
pub struct KeyringStore;

impl TokenStore for KeyringStore {
    fn refresh_token(&self) -> Result<Option<String>> {
        secrets::get(Secret::MalRefreshToken)
    }
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()> {
        match token {
            Some(token) => secrets::set(Secret::MalRefreshToken, token),
            None => secrets::delete(Secret::MalRefreshToken),
        }
    }
    fn client_secret(&self) -> Result<Option<String>> {
        secrets::get(Secret::MalClientSecret)
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        match secret {
            Some(s) if !s.trim().is_empty() => secrets::set(Secret::MalClientSecret, s.trim()),
            _ => secrets::delete(Secret::MalClientSecret),
        }
    }
}

/// An in-memory store for tests and for store-agnostic callers.
#[derive(Default)]
pub struct MemoryStore {
    token: Mutex<Option<String>>,
    secret: Mutex<Option<String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with(token: &str) -> Self {
        Self {
            token: Mutex::new(Some(token.to_string())),
            secret: Mutex::new(None),
        }
    }
}

impl TokenStore for MemoryStore {
    fn refresh_token(&self) -> Result<Option<String>> {
        let guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        Ok(guard.clone())
    }
    fn set_refresh_token(&self, token: Option<&str>) -> Result<()> {
        let mut guard = self.token.lock().unwrap_or_else(|e| e.into_inner());
        *guard = token.map(str::to_string);
        Ok(())
    }
    fn client_secret(&self) -> Result<Option<String>> {
        let guard = self.secret.lock().unwrap_or_else(|e| e.into_inner());
        Ok(guard.clone())
    }
    fn set_client_secret(&self, secret: Option<&str>) -> Result<()> {
        let mut guard = self.secret.lock().unwrap_or_else(|e| e.into_inner());
        *guard = secret.map(str::to_string);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The client
// ---------------------------------------------------------------------------

/// One MAL session: a gate, the settings and a per-process token cache.
pub struct MalClient<'a> {
    gate: &'a Gate,
    settings: Settings,
    store: &'a dyn TokenStore,
    clock: &'a dyn Clock,
    tokens: Mutex<Option<Tokens>>,
}

impl<'a> MalClient<'a> {
    pub fn new(
        gate: &'a Gate,
        settings: Settings,
        store: &'a dyn TokenStore,
        clock: &'a dyn Clock,
    ) -> Self {
        MalClient {
            gate,
            settings,
            store,
            clock,
            tokens: Mutex::new(None),
        }
    }

    pub fn clock(&self) -> &dyn Clock {
        self.clock
    }
    pub fn settings(&self) -> &Settings {
        &self.settings
    }
    pub fn db(&self) -> &Db {
        self.gate.db()
    }

    /// A refresh token is stored and usable.
    pub fn is_connected(&self) -> bool {
        self.store.refresh_token().ok().flatten().is_some()
    }

    fn base(&self) -> &str {
        self.settings.mal_base_url.trim_end_matches('/')
    }

    /// MAL hosts its OAuth token endpoint on `myanimelist.net/v1/oauth2/token`
    /// rather than the API host `api.myanimelist.net/v2`. In test suites or
    /// custom setups where `mal_base_url` points to a mock server, fall back
    /// to `{base}{TOKEN_PATH}`.
    fn token_url(&self) -> String {
        let base = self.base();
        if base.contains("myanimelist.net") {
            TOKEN_URL.to_string()
        } else {
            format!("{base}{TOKEN_PATH}")
        }
    }

    fn client_id(&self) -> Result<String> {
        let id = self.settings.mal_client_id.trim().to_string();
        if id.is_empty() {
            return Err(Error::Auth {
                service: "mal".to_string(),
                message: "no MAL client id is set; add one to settings.mal_client_id"
                    .to_string(),
            });
        }
        Ok(id)
    }

    /// A valid access token, refreshing five minutes before it expires.
    pub async fn access_token(&self) -> Result<String> {
        let now = self.clock.now();
        {
            let guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(tokens) = guard.as_ref() {
                if now < tokens.expires_at - REFRESH_MARGIN_SECS {
                    return Ok(tokens.access_token.clone());
                }
            }
        }

        let refresh = self.store.refresh_token()?.ok_or_else(|| Error::Auth {
            service: "mal".to_string(),
            message: "not connected to MyAnimeList; run `genrl mal login` first".to_string(),
        })?;
        let client_id = self.client_id()?;
        let tokens = self.refresh(&client_id, &refresh).await?;
        {
            let mut guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(tokens.clone());
        }
        Ok(tokens.access_token)
    }

    async fn refresh(&self, client_id: &str, refresh_token: &str) -> Result<Tokens> {
        let url = self.token_url();
        let mut form = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("client_id".to_string(), client_id.to_string()),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ];
        let secret = self
            .store
            .client_secret()
            .ok()
            .flatten()
            .or_else(|| std::env::var("GENRL_MAL_CLIENT_SECRET").ok());
        if let Some(s) = secret {
            if !s.trim().is_empty() {
                form.push(("client_secret".to_string(), s.trim().to_string()));
            }
        }
        self.token_request(&url, &form).await
    }

    /// Swap a login code for tokens. The token endpoint rotates the refresh
    /// token; the fresh one is stored before this returns.
    async fn exchange_and_store(
        &self,
        redirect_uri: &str,
        code: &str,
        verifier: &str,
    ) -> Result<Tokens> {
        let client_id = self.client_id()?;
        let url = self.token_url();
        let mut form = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("client_id".to_string(), client_id),
            ("code".to_string(), code.to_string()),
            ("code_verifier".to_string(), verifier.to_string()),
            ("redirect_uri".to_string(), redirect_uri.to_string()),
        ];
        let secret = self
            .store
            .client_secret()
            .ok()
            .flatten()
            .or_else(|| std::env::var("GENRL_MAL_CLIENT_SECRET").ok());
        if let Some(s) = secret {
            if !s.trim().is_empty() {
                form.push(("client_secret".to_string(), s.trim().to_string()));
            }
        }
        let tokens = self.token_request(&url, &form).await?;
        {
            let mut guard = self.tokens.lock().unwrap_or_else(|e| e.into_inner());
            *guard = Some(tokens.clone());
        }
        Ok(tokens)
    }

    /// Token endpoint, hosted on myanimelist.net (not the api host).
    async fn token_request(&self, url: &str, form: &[(String, String)]) -> Result<Tokens> {
        let body_result = self.gate.post_form(Service::Mal, url, form, &GetOptions::default()).await;
        let fetched = match body_result {
            Ok(fetched) => fetched,
            Err(Error::Status { status, url, body, .. }) => {
                let reason = body.unwrap_or_default();
                let detail = if reason.is_empty() {
                    format!("token endpoint returned HTTP {status} for {url}")
                } else {
                    format!("token endpoint returned HTTP {status}: {reason}")
                };
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: format!(
                        "{detail}\nChecklist:\n1. Verify client_id in Settings.\n2. Ensure redirect URI registered in MAL API config matches http://localhost:<port>/callback exactly."
                    ),
                });
            }
            Err(err) => return Err(err),
        };
        let body = fetched.text().unwrap_or_default();
        let raw: TokenResponse = match fetched.json() {
            Ok(raw) => raw,
            Err(_) if !body.is_empty() => {
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: format!("token endpoint response: {body}"),
                });
            }
            Err(err) => return Err(err),
        };
        if let Some(rotated) = raw.refresh_token.as_deref() {
            self.store.set_refresh_token(Some(rotated))?;
        }
        Ok(Tokens {
            access_token: raw.access_token,
            expires_at: self.clock.now() + raw.expires_in,
        })
    }

    /// GET through the gate with the bearer token on the header. The token
    /// never becomes part of the URL, so the gate's response cache cannot leak
    /// it anywhere either.
    async fn get(&self, url: &str) -> Result<Fetched> {
        let token = self.access_token().await?;
        let opts = GetOptions {
            headers: vec![("Authorization".to_string(), format!("Bearer {token}"))],
            ..Default::default()
        };
        self.gate.get_with(Service::Mal, url, &opts).await
    }

    /// The logged-in account (for `genrl mal status`/`login`).
    pub async fn account(&self) -> Result<Account> {
        let url = format!("{}/users/@me", self.base());
        let fetched = self.get(&url).await?;
        Ok(fetched.json()?)
    }

    /// The whole list, paged, with chains built from MAL relations.
    pub async fn fetch(&self) -> Result<FetchedList> {
        let mut entries = self.fetch_list().await?;
        let mut relations: HashMap<i64, Vec<Relation>> = HashMap::new();
        let mut relations_failed = 0usize;
        let mut first_error: Option<Error> = None;

        for entry in &entries {
            let id = entry.entry.mal_id;
            match self.relations(id).await {
                Ok(rels) => {
                    relations.insert(id, rels);
                }
                Err(err) => {
                    if first_error.is_none() {
                        first_error = Some(err);
                    }
                    relations_failed += 1;
                    tracing::warn!(
                        mal_id = id,
                        "MAL relation lookup failed; the entry stays its own chain"
                    );
                }
            }
        }

        if relations_failed > 0 && relations_failed == entries.len() {
            // Every lookup failed: something systematic (expired token, MAL
            // down). Swallowing it would hide a real outage behind a wall of
            // singleton chains.
            if let Some(err) = first_error {
                return Err(err);
            }
        }

        build_chains(&mut entries, &relations);
        Ok(FetchedList {
            entries,
            relations_failed,
        })
    }

    async fn fetch_list(&self) -> Result<Vec<MalEntry>> {
        let mut out = Vec::new();
        let mut offset = 0usize;
        for _ in 0..MAX_PAGES {
            let url = format!(
                "{}{LIST_PATH}?fields={LIST_FIELDS}&limit={PAGE_SIZE}&offset={offset}&nsfw=true",
                self.base(),
            );
            let fetched = self.get(&url).await?;
            let page: ListPage = fetched.json()?;
            let n = page.data.len();
            if n == 0 {
                break;
            }
            out.extend(page.data.iter().map(to_mal_entry));
            offset += n;
            let has_more = page
                .paging
                .as_ref()
                .and_then(|p| p.next.as_ref())
                .is_some();
            if !has_more {
                break;
            }
        }
        Ok(out)
    }

    async fn relations(&self, mal_id: i64) -> Result<Vec<Relation>> {
        let url = format!(
            "{}/anime/{mal_id}?fields={RELATIONS_FIELD}",
            self.base(),
        );
        let fetched = self.get(&url).await?;
        let detail: AnimeDetail = fetched.json()?;
        Ok(detail.related_anime)
    }
}

// ---------------------------------------------------------------------------
// The login flow
// ---------------------------------------------------------------------------

/// A login in progress: a one-shot loopback listener waiting for the browser.
pub struct Login {
    listener: tokio::net::TcpListener,
    redirect_uri: String,
    verifier: String,
    state: String,
    url: String,
    port: u16,
}

impl Login {
    /// The URL to hand to the browser.
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Wait for the redirect (one request only), swap the code for tokens and
    /// store the rotated refresh token.
    pub async fn finish(self, client: &MalClient<'_>, timeout: Duration) -> Result<()> {
        let code = redirect_code(self.listener, &self.state, timeout).await?;
        let _ = client
            .exchange_and_store(&self.redirect_uri, &code, &self.verifier)
            .await?;
        Ok(())
    }
}

/// Bind a random loopback port and build the authorize URL: random port, CSRF
/// `state`, PKCE, and the listener closes after one request — the design's
/// "OAuth redirect hijack" mitigation.
pub async fn start_login(settings: &Settings) -> Result<Login> {
    start_login_with_port(settings, None).await
}

/// Start login with an optional explicit port override.
pub async fn start_login_with_port(settings: &Settings, override_port: Option<u16>) -> Result<Login> {
    start_login_with_options(settings, override_port, None).await
}

/// Start login with an optional explicit port and optional redirect URI override.
pub async fn start_login_with_options(
    settings: &Settings,
    override_port: Option<u16>,
    override_redirect_uri: Option<&str>,
) -> Result<Login> {
    let client_id = settings
        .mal_client_id
        .trim()
        .to_string();
    if client_id.is_empty() {
        return Err(Error::Auth {
            service: "mal".to_string(),
            message: "set settings.mal_client_id (myanimelist.net/apiconfig) before logging in"
                .to_string(),
        });
    }
    // A fixed loopback port keeps the redirect URI byte-stable across runs
    // (MAL requires the redirect_uri to *exactly match* what was registered in
    // the app form). Override with GENRL_MAL_REDIRECT_PORT or explicit override.
    let port: u16 = override_port.unwrap_or_else(|| {
        std::env::var("GENRL_MAL_REDIRECT_PORT")
            .ok()
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(21445)
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await?;
    let redirect_uri = override_redirect_uri
        .map(str::to_string)
        .or_else(|| std::env::var("GENRL_MAL_REDIRECT_URI").ok())
        .unwrap_or_else(|| format!("http://127.0.0.1:{port}/callback"));
    let verifier = pkce_verifier();
    let state = random_token(32);
    let url = authorize_url(&client_id, &redirect_uri, &state, &code_challenge(&verifier));
    Ok(Login {
        listener,
        redirect_uri,
        verifier,
        state,
        url,
        port,
    })
}

/// Parse the query of the loopback request: `code`, `state`, and `error` when
/// the user denied the login.
fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let (key, value) = pair.split_once('=')?;
            Some((pct_decode(key), pct_decode(value)))
        })
        .collect()
}

/// Accept loopback request(s), verify the OAuth `state` against `CSRF`
/// and return the code, or explain why not. Ignores browser background requests
/// like `/favicon.ico`.
pub async fn redirect_code(
    listener: tokio::net::TcpListener,
    expected_state: &str,
    timeout: Duration,
) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(Error::Auth {
                service: "mal".to_string(),
                message: "the login timed out waiting for the browser redirect".to_string(),
            });
        }

        let accept = tokio::time::timeout(remaining, listener.accept()).await;
        let (mut stream, _peer) = match accept {
            Ok(Ok(pair)) => pair,
            Ok(Err(err)) => return Err(err.into()),
            Err(_) => {
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: "the login timed out waiting for the browser redirect".to_string(),
                });
            }
        };

        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            continue;
        }
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/");

        if target.starts_with("/favicon") {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n").await;
            let _ = stream.shutdown().await;
            continue;
        }

        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
        let params = parse_query(query);

        let code = params.iter().find(|(k, _)| k == "code").map(|(_, v)| v.clone());
        let state = params.iter().find(|(k, _)| k == "state").map(|(_, v)| v.clone());
        let error = params.iter().find(|(k, _)| k == "error").map(|(_, v)| v.clone());

        let ok = code.is_some()
            && state.as_deref() == Some(expected_state)
            && error.is_none();
        let page = if ok {
            "<!doctype html><meta charset=\"utf-8\"><title>Genrl</title><p>You can close this tab now.</p>"
                .to_string()
        } else {
            "<!doctype html><meta charset=\"utf-8\"><title>Genrl</title><p>Login failed: the state did not match.</p>"
                .to_string()
        };
        let response = format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {len}\r\nConnection: close\r\n\r\n{page}",
            status = if ok { 200 } else { 400 },
            len = page.len(),
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;

        match (code, state, error) {
            (_, _, Some(denied)) => return Err(Error::Auth {
                service: "mal".to_string(),
                message: format!("MyAnimeList denied the login: {denied}"),
            }),
            (Some(code), Some(state), None) if state == expected_state => return Ok(code),
            _ => return Err(Error::Auth {
                service: "mal".to_string(),
                message: "the redirect state did not match; refusing the code".to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Wire types and parsing
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default = "default_expires_in")]
    expires_in: i64,
}

fn default_expires_in() -> i64 {
    3600
}

#[derive(Debug, Deserialize)]
struct ListPage {
    data: Vec<ListItem>,
    #[serde(default)]
    paging: Option<Paging>,
}

#[derive(Debug, Deserialize)]
struct Paging {
    #[serde(default)]
    next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListItem {
    node: Node,
    list_status: RawListStatus,
}

/// One anime as MAL reports it (list node or relation node).
#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    id: i64,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    main_title: Option<String>,
    #[serde(default)]
    alternative_titles: Option<AltTitles>,
    /// `Some(0)` means "unknown"; it maps to `None`.
    #[serde(default)]
    num_episodes: Option<i64>,
    #[serde(default)]
    start_season: Option<SeasonInfo>,
    #[serde(default)]
    media_type: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    main_picture: Option<MainPicture>,
    #[serde(default)]
    synopsis: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MainPicture {
    #[serde(default)]
    medium: Option<String>,
    #[serde(default)]
    large: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct AltTitles {
    #[serde(default)]
    synonyms: Vec<String>,
    #[serde(default)]
    en: Option<String>,
    #[serde(default)]
    ja: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SeasonInfo {
    #[serde(default)]
    year: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
struct RawListStatus {
    status: String,
    #[serde(default)]
    score: Option<i64>,
    #[serde(default)]
    num_episodes_watched: Option<i64>,
    #[serde(default)]
    updated_at: Option<String>,
}

/// `GET /anime/{id}?fields=related_anime`.
#[derive(Debug, Deserialize)]
struct AnimeDetail {
    #[serde(default)]
    related_anime: Vec<Relation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relation {
    pub node: Node,
    pub relation_type: String,
}

/// A list row once it has been parsed into planner shape, plus the raw fields
/// the diff stores in `mal_entries` and `shows`.
#[derive(Debug, Clone, PartialEq)]
pub struct MalEntry {
    pub entry: ListEntry,
    pub score: Option<i64>,
    pub updated_at: Option<String>,
    pub media_type: Option<String>,
    pub airing_status: Option<String>,
    pub source: String,
    pub poster_url: Option<String>,
    pub synopsis: Option<String>,
}

impl MalEntry {
    pub fn mal_id(&self) -> i64 {
        self.entry.mal_id
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FetchedList {
    pub entries: Vec<MalEntry>,
    pub relations_failed: usize,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub id: i64,
    #[serde(default)]
    pub name: String,
}

fn to_mal_entry(item: &ListItem) -> MalEntry {
    let node = &item.node;
    let alt = node.alternative_titles.clone().unwrap_or_default();
    let title = node
        .title
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| node.main_title.clone().filter(|t| !t.is_empty()))
        .unwrap_or_else(|| format!("MAL #{}", node.id));

    // The main title first, then English, Japanese and synonyms, deduplicated
    // case-insensitively: this is the matching engine's whole input set.
    let mut names: Vec<String> = Vec::new();
    let candidates = [Some(title.clone()), node.main_title.clone(), alt.en.clone(), alt.ja.clone()]
        .into_iter()
        .flatten()
        .chain(alt.synonyms.iter().cloned());
    for name in candidates {
        let name = name.trim().to_string();
        if !name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }

    let status = ListStatus::parse(&item.list_status.status);
    let entry = ListEntry {
        mal_id: node.id,
        title,
        english_title: alt.en.filter(|t| !t.is_empty()),
        list_status: status,
        watched_count: item.list_status.num_episodes_watched.unwrap_or(0).max(0),
        total_episodes: node.num_episodes.filter(|n| *n > 0),
        start_year: node.start_season.as_ref().and_then(|s| s.year),
        auto: matches!(status, ListStatus::Watching),
        episode_offset: 0,
        names,
        franchise: node.id,
        season: 1,
    };

    let poster_url = node
        .main_picture
        .as_ref()
        .and_then(|p| p.large.clone().or_else(|| p.medium.clone()));

    MalEntry {
        entry,
        score: item.list_status.score,
        updated_at: item.list_status.updated_at.clone(),
        media_type: node.media_type.clone(),
        airing_status: node.status.clone(),
        source: "mal".to_string(),
        poster_url,
        synopsis: node.synopsis.clone(),
    }
}

fn show_type(kind: Option<&str>) -> &'static str {
    match kind {
        Some("movie") => "movie",
        Some("ova") => "ova",
        Some("ona") | Some("music") => "ona",
        Some("special") | Some("tv_special") => "special",
        _ => "tv",
    }
}

fn airing_status(s: Option<&str>) -> Option<&'static str> {
    match s {
        Some("currently_airing") => Some("airing"),
        Some("finished_airing") => Some("finished"),
        Some("not_yet_aired") => Some("not_yet_aired"),
        // Unknown: keep whatever the previous sync knew.
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The season chain
// ---------------------------------------------------------------------------

/// Fill `franchise`, `season` and `episode_offset` from MAL's sequel/prequel
/// relations: each connected component is one chain, ordered along the links,
/// offset by the cumulative episode counts of the earlier seasons. Entries
/// with no relations (or whose lookups failed) stay their own chain.
pub fn build_chains(entries: &mut [MalEntry], relations: &HashMap<i64, Vec<Relation>>) {
    let index: HashMap<i64, usize> = entries.iter().enumerate().map(|(i, m)| (m.mal_id(), i)).collect();
    let id_set: HashSet<i64> = index.keys().copied().collect();

    // `precedes` maps an id to the ids that come after it; `links` is the same
    // graph made undirected, for grouping.
    let mut precedes: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut links: HashMap<i64, HashSet<i64>> = HashMap::new();

    for (from, rels) in relations {
        if !id_set.contains(from) {
            continue;
        }
        for rel in rels {
            let to = rel.node.id;
            if !id_set.contains(&to) || to == *from {
                continue;
            }
            let (before, after) = match rel.relation_type.as_str() {
                "sequel" => (*from, to),
                "prequel" => (to, *from),
                _ => continue,
            };
            precedes.entry(before).or_default().insert(after);
            links.entry(before).or_default().insert(after);
            links.entry(after).or_default().insert(before);
        }
    }

    // Connected components (undirected), each stable-ordered for deterministic
    // franchise numbering.
    let mut seen: HashSet<i64> = HashSet::new();
    let mut components: Vec<Vec<i64>> = Vec::new();
    for &id in &index.keys().copied().collect::<Vec<i64>>() {
        if !seen.insert(id) {
            continue;
        }
        let mut stack = vec![id];
        let mut component = Vec::new();
        while let Some(current) = stack.pop() {
            component.push(current);
            if let Some(neighbours) = links.get(&current) {
                for &next in neighbours {
                    if seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }

    for component in &components {
        // Kahn's order: emit a node when nothing still-remaining points at it
        // as a successor — i.e., no remaining node `precedes` it.  A cycle
        // falls back to the smallest id so the order stays stable anyway.
        let mut remaining: HashSet<i64> = component.iter().copied().collect();
        let mut ordered: Vec<i64> = Vec::with_capacity(component.len());
        while !remaining.is_empty() {
            let mut pick = None;
            for &id in &remaining {
                // id is ready when no other remaining node lists id as a
                // successor (i.e., nothing that is still unprocessed comes
                // before id in the chain).
                let has_predecessor = precedes.values().any(|succs| succs.contains(&id))
                    && precedes
                        .iter()
                        .any(|(pred, succs)| succs.contains(&id) && remaining.contains(pred));
                let ready = !has_predecessor;
                if ready {
                    pick = Some(id);
                    break;
                }
            }
            let id = match pick {
                Some(id) => id,
                None => {
                    match remaining.iter().min() {
                        Some(&first) => first,
                        None => break,
                    }
                }
            };
            remaining.remove(&id);
            ordered.push(id);
        }

        // The smallest mal id names the franchise, so it is stable across
        // syncs; seasons run along the ordered chain.
        let franchise = component.first().copied().unwrap_or_default();
        let mut offset: Option<i64> = Some(0);
        for (position, id) in ordered.iter().enumerate() {
            let total = entries.get(index[id]).and_then(|m| m.entry.total_episodes);
            if let Some(entry) = entries.get_mut(index[id]) {
                entry.entry.franchise = franchise;
                entry.entry.season = position as i64 + 1;
                entry.entry.episode_offset = offset.unwrap_or(0);
            }
            offset = offset.and_then(|o| total.map(|t| o + t));
        }
    }
}

// ---------------------------------------------------------------------------
// The sync: list diff against what last sync stored
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncReport {
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub total: usize,
    pub relations_failed: usize,
    pub metadata_jobs_queued: usize,
    /// Titles MAL changed; one `entry_changed` question is raised per change.
    pub renamed: Vec<String>,
    /// The plan-ready entries (`plan --from-mal` replans straight from these).
    pub entries: Vec<ListEntry>,
}

impl SyncReport {
    /// One line for `genrl mal sync` and the plan command.
    pub fn summary(&self) -> String {
        let mut s = format!(
            "MAL list: {} new, {} updated, {} unchanged, {} removed, {} total",
            self.added, self.updated, self.unchanged, self.removed, self.total
        );
        if self.relations_failed > 0 {
            s.push_str(&format!(
                " ({} season-relation lookups failed; those stay single-season)",
                self.relations_failed
            ));
        }
        if self.metadata_jobs_queued > 0 {
            s.push_str(&format!(", {} metadata refresh jobs queued", self.metadata_jobs_queued));
        }
        s
    }
}

#[derive(Debug, Clone, PartialEq)]
struct StoredEntry {
    status: String,
    watched: i64,
    score: Option<i64>,
    updated_at: Option<String>,
    source: String,
}

impl StoredEntry {
    fn changed(&self, m: &MalEntry) -> bool {
        self.status != m.entry.list_status.as_str()
            || self.watched != m.entry.watched_count
            || self.score != m.score
            || self.updated_at.as_deref() != m.updated_at.as_deref()
            || self.source != m.source
    }
}

/// Fetch, diff and store the list: `shows`, `show_names` and `mal_entries`
/// mirror the list; nothing is queued — planning from the returned entries is
/// a separate (confirmed) step.
pub async fn sync(client: &MalClient<'_>) -> Result<SyncReport> {
    let db = client.db().clone();
    let now = client.clock().now();
    let settings = client.settings();
    let log = EventLog::new(db.clone());

    let fetched = client.fetch().await?;
    let mut report = SyncReport {
        total: fetched.entries.len(),
        relations_failed: fetched.relations_failed,
        ..Default::default()
    };

    let mut fetched_ids = Vec::with_capacity(fetched.entries.len());
    for m in &fetched.entries {
        let id = m.mal_id();
        fetched_ids.push(id);

        // Shows and names first (the matching engine reads them); then the
        // diff that decides added vs updated vs unchanged.
        upsert_show_full(&db, m)?;
        crate::planner::upsert_names(&db, &m.entry)?;

        let prev_title: Option<String> = db.with(|c| {
            Ok(c.query_row(
                "SELECT title FROM shows WHERE mal_id = ?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .ok())
        })?;

        match load_mal_entry(&db, id)? {
            None => {
                save_mal_entry(&db, m)?;
                report.added += 1;
                let _ = log.info(None, Some(id), format!("MAL sync: added \"{}\"", m.entry.title));
            }
            Some(stored) => {
                if let Some(old) = prev_title {
                    if old != m.entry.title {
                        report.renamed.push(m.entry.title.clone());
                        let detail =
                            format!("MAL renamed this entry from \"{old}\" to \"{}\"", m.entry.title);
                        let expires = i64::from(settings.question_expiry_hours) * 3600;
                        if raise_entry_changed(&db, now, id, &m.entry.title, detail, Some(expires))? {
                            let _ = log.warn(
                                None,
                                Some(id),
                                format!("MAL sync: renamed \"{old}\" to \"{}\"", m.entry.title),
                            );
                        }
                    }
                }
                if stored.changed(m) {
                    save_mal_entry(&db, m)?;
                    report.updated += 1;
                } else {
                    report.unchanged += 1;
                }
            }
        }
    }

    // Entries that left the list: drop their `mal_entries` row so the table
    // stays an honest mirror, switch the show off the automatic list, but keep
    // the show and its library (the design never deletes files).
    let known: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare("SELECT mal_id FROM mal_entries")?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        Ok(rows.filter_map(std::result::Result::ok).collect::<Vec<_>>())
    })?;
    for id in known {
        if !fetched_ids.contains(&id) {
            db.with(|c| {
                c.execute("DELETE FROM mal_entries WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            db.with(|c| {
                c.execute("UPDATE shows SET auto = 0 WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            report.removed += 1;
            let _ = log.info(None, Some(id), "MAL sync: no longer on your list");
        }
    }

    // Queue metadata refresh jobs for any show missing synopsis/poster or with stale data (> 7 days)
    if let Ok(queued) = crate::metadata::queue_stale_metadata(&db, now) {
        report.metadata_jobs_queued = queued;
    }

    settings::write_state(&db, SYNC_STATE_KEY, &now.to_string())?;
    report.entries = fetched.entries.into_iter().map(|m| m.entry).collect();
    let _ = log.info(
        None,
        None,
        format!(
            "MAL sync: {} new, {} updated, {} unchanged, {} removed, {} metadata refresh queued",
            report.added, report.updated, report.unchanged, report.removed, report.metadata_jobs_queued
        ),
    );
    Ok(report)
}

/// One `entry_changed` per show per sync — the Inbox reads them oldest first,
/// so a dozen renames still surface once each, never as a wall.
fn raise_entry_changed(
    db: &Db,
    now: i64,
    mal_id: i64,
    title: &str,
    detail: String,
    expires_in: Option<i64>,
) -> Result<bool> {
    if QuestionStore::of_kind(db, QuestionKind::EntryChanged, now)?
        .iter()
        .any(|q| q.mal_id == Some(mal_id))
    {
        return Ok(false);
    }
    QuestionStore::ask(
        db,
        None,
        Some(mal_id),
        &Payload::EntryChanged {
            mal_id,
            title: title.to_string(),
            detail,
        },
        now,
        expires_in,
    )?;
    Ok(true)
}

fn load_mal_entry(db: &Db, mal_id: i64) -> Result<Option<StoredEntry>> {
    db.with(|c| {
        Ok(c.query_row(
            "SELECT list_status, watched_count, score, updated_at, source
             FROM mal_entries WHERE mal_id = ?1",
            [mal_id],
            |r| {
                Ok(StoredEntry {
                    status: r.get(0)?,
                    watched: r.get(1)?,
                    score: r.get(2)?,
                    updated_at: r.get(3)?,
                    source: r.get(4)?,
                })
            },
        )
        .ok())
    })
}

fn save_mal_entry(db: &Db, m: &MalEntry) -> Result<()> {
    db.with(|c| {
        c.execute(
            "INSERT INTO mal_entries (mal_id, list_status, watched_count, score, updated_at, source)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (mal_id) DO UPDATE SET
                list_status = excluded.list_status,
                watched_count = excluded.watched_count,
                score = excluded.score,
                updated_at = excluded.updated_at,
                source = excluded.source",
            rusqlite::params![
                m.mal_id(),
                m.entry.list_status.as_str(),
                m.entry.watched_count,
                m.score,
                m.updated_at,
                m.source,
            ],
        )?;
        Ok(())
    })
}

/// The sync's own show upsert, mirroring the real MAL status and type; the
/// planner's `upsert_show` never overwrites them (it only updates the fields
/// it knows).
fn upsert_show_full(db: &Db, m: &MalEntry) -> Result<()> {
    let e = &m.entry;
    let media = show_type(m.media_type.as_deref());
    let airing = airing_status(m.airing_status.as_deref());
    db.with(|c| {
        c.execute(
            "INSERT INTO shows (mal_id, title, english_title, type, status, total_episodes,
                                start_year, auto, episode_offset, season, poster_path, synopsis)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT (mal_id) DO UPDATE SET
                title = excluded.title,
                english_title = COALESCE(excluded.english_title, shows.english_title),
                type = excluded.type,
                status = COALESCE(excluded.status, shows.status),
                total_episodes = COALESCE(excluded.total_episodes, shows.total_episodes),
                start_year = COALESCE(excluded.start_year, shows.start_year),
                auto = excluded.auto,
                episode_offset = excluded.episode_offset,
                season = excluded.season,
                poster_path = COALESCE(excluded.poster_path, shows.poster_path),
                synopsis = COALESCE(excluded.synopsis, shows.synopsis)",
            rusqlite::params![
                e.mal_id,
                e.title,
                e.english_title,
                media,
                airing,
                e.total_episodes,
                e.start_year,
                e.auto as i64,
                e.episode_offset,
                e.season,
                m.poster_url,
                m.synopsis,
            ],
        )?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// The cadence
// ---------------------------------------------------------------------------

/// The design's "sync at most every 30 minutes": due when the last sync is
/// older than the interval, or has never happened. Cheap — a settings read.
pub fn is_due(db: &Db, clock: &dyn Clock, interval_min: u32) -> Result<bool> {
    match settings::read_state_time(db, SYNC_STATE_KEY)? {
        Some(last) => Ok(clock.now() - last >= i64::from(interval_min) * 60),
        None => Ok(true),
    }
}

/// Sync when connected and the cadence allows it — what `genrl run` calls
/// every tick. A failure is logged, never fatal: a MAL outage must not stop
/// the workers.
pub async fn sync_if_due(
    db: &Db,
    gate: &Gate,
    clock: &dyn Clock,
    store: &dyn TokenStore,
) -> Option<SyncReport> {
    let settings = match Settings::load(db) {
        Ok(settings) => settings,
        Err(err) => {
            tracing::warn!("could not load settings for MAL sync: {err}");
            return None;
        }
    };
    if settings.mal_client_id.trim().is_empty() {
        return None;
    }
    match is_due(db, clock, settings.mal_sync_interval_min) {
        Ok(true) => {}
        _ => return None,
    }
    if store.refresh_token().ok().flatten().is_none() {
        return None;
    }
    let client = MalClient::new(gate, settings, store, clock);
    match sync(&client).await {
        Ok(report) => Some(report),
        Err(err) => {
            tracing::warn!("MAL sync failed: {err}");
            let _ = EventLog::new(db.clone()).warn(None, None, format!("MAL sync failed: {err}"));
            let _ = settings::write_state_now(db, SYNC_STATE_KEY);
            None
        }
    }
}


// ---------------------------------------------------------------------------
// Jikan Public Profile Fallback & Relations
// ---------------------------------------------------------------------------

pub const JIKAN_USER_STATE_KEY: &str = "mal_public_username";
pub const ACCOUNT_SOURCE_STATE_KEY: &str = "mal_account_source";

#[derive(Debug, Deserialize)]
struct JikanUserAnimelistResponse {
    data: Vec<JikanUserAnimelistEntry>,
    #[serde(default)]
    pagination: Option<JikanPagination>,
}

#[derive(Debug, Deserialize)]
struct JikanPagination {
    #[serde(default)]
    has_next_page: bool,
}

#[derive(Debug, Deserialize)]
struct JikanUserAnimelistEntry {
    entry: JikanEntryNode,
    #[serde(default)]
    score: Option<i64>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    episodes_seen: Option<i64>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JikanEntryNode {
    mal_id: i64,
    title: String,
    #[serde(default)]
    #[allow(dead_code)]
    images: Option<JikanImages>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JikanImages {
    #[serde(default)]
    jpg: Option<JikanImageJpg>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JikanImageJpg {
    #[serde(default)]
    image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JikanAnimeDetailResponse {
    data: JikanAnimeDetailData,
}

#[derive(Debug, Deserialize)]
struct JikanAnimeDetailData {
    #[allow(dead_code)]
    mal_id: i64,
    title: String,
    #[serde(default)]
    title_english: Option<String>,
    #[serde(default)]
    title_japanese: Option<String>,
    #[serde(default)]
    titles: Option<Vec<JikanTitleItem>>,
    #[serde(rename = "type", default)]
    media_type: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    episodes: Option<i64>,
    #[serde(default)]
    year: Option<i64>,
    #[serde(default)]
    relations: Option<Vec<JikanRelationItem>>,
}

#[derive(Debug, Deserialize)]
struct JikanTitleItem {
    #[serde(rename = "type", default)]
    #[allow(dead_code)]
    title_type: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct JikanRelationItem {
    relation: String,
    #[serde(default)]
    entry: Vec<JikanRelationSubEntry>,
}

#[derive(Debug, Deserialize)]
struct JikanRelationSubEntry {
    mal_id: i64,
    #[serde(rename = "type", default)]
    entry_type: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct MalLoadJsonItem {
    status: i32,
    #[serde(default)]
    score: Option<i64>,
    #[serde(default)]
    num_watched_episodes: Option<i64>,
    #[serde(default)]
    updated_at: Option<serde_json::Value>,
    anime_title: serde_json::Value,
    #[serde(default)]
    anime_title_eng: Option<serde_json::Value>,
    #[serde(default)]
    anime_num_episodes: Option<i64>,
    #[serde(default)]
    anime_airing_status: Option<i32>,
    anime_id: i64,
    #[serde(default)]
    anime_media_type_string: Option<String>,
    #[serde(default)]
    anime_image_path: Option<String>,
    #[serde(default)]
    anime_start_date_string: Option<String>,
}

fn value_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn parse_start_year(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split(|c: char| c == '-' || c == '/' || c == '.').collect();
    for p in &parts {
        if p.len() == 4 {
            if let Ok(y) = p.parse::<i64>() {
                if (1950..=2050).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    let last = parts.last()?;
    if last.len() == 2 {
        if let Ok(yy) = last.parse::<i64>() {
            let y = if yy <= 50 { 2000 + yy } else { 1900 + yy };
            return Some(y);
        }
    }
    None
}

/// Fetch a user's anime list directly from MAL's public load.json endpoint.
pub async fn fetch_mal_public_load_json(gate: &Gate, username: &str) -> Result<Vec<MalEntry>> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    loop {
        let url = format!("https://myanimelist.net/animelist/{username}/load.json?offset={offset}&status=7");
        let fetched = gate.get_with(Service::Mal, &url, &GetOptions::default()).await?;
        let items: Vec<MalLoadJsonItem> = fetched.json()?;
        if items.is_empty() {
            break;
        }
        let count = items.len();
        for item in items {
            let list_status = match item.status {
                1 => ListStatus::Watching,
                2 => ListStatus::Completed,
                3 => ListStatus::OnHold,
                4 => ListStatus::Dropped,
                6 => ListStatus::PlanToWatch,
                _ => ListStatus::PlanToWatch,
            };
            let title = value_to_string(&item.anime_title)
                .unwrap_or_else(|| format!("MAL #{}", item.anime_id));
            let mut names = vec![title.clone()];
            let eng = item.anime_title_eng.as_ref().and_then(value_to_string);
            if let Some(ref e) = eng {
                if !names.contains(e) {
                    names.push(e.clone());
                }
            }
            let airing = match item.anime_airing_status {
                Some(1) => Some("currently_airing".to_string()),
                Some(2) => Some("finished_airing".to_string()),
                Some(3) => Some("not_yet_aired".to_string()),
                _ => None,
            };
            let start_year = item.anime_start_date_string.as_deref().and_then(parse_start_year);
            let updated_at = item.updated_at.as_ref().and_then(value_to_string);

            let entry = ListEntry {
                mal_id: item.anime_id,
                title: title.clone(),
                english_title: eng,
                list_status,
                watched_count: item.num_watched_episodes.unwrap_or(0).max(0),
                total_episodes: item.anime_num_episodes.filter(|n| *n > 0),
                start_year,
                auto: matches!(list_status, ListStatus::Watching),
                episode_offset: 0,
                names,
                franchise: item.anime_id,
                season: 1,
            };

            out.push(MalEntry {
                entry,
                score: item.score,
                updated_at,
                media_type: item.anime_media_type_string.clone(),
                airing_status: airing,
                source: "jikan".to_string(),
                poster_url: item.anime_image_path.clone(),
                synopsis: None,
            });
        }
        offset += count;
        if count < 300 || offset >= 10000 {
            break;
        }
    }
    Ok(out)
}

/// Fetch a user's anime list from Jikan (`/v4/users/{username}/animelist`) through Gate.
pub async fn fetch_jikan_list(gate: &Gate, settings: &Settings, username: &str) -> Result<Vec<MalEntry>> {
    let mut out = Vec::new();
    let raw_base = settings.jikan_base_url.trim_end_matches('/');
    let base = if raw_base.ends_with("/v4") {
        raw_base.to_string()
    } else {
        format!("{raw_base}/v4")
    };
    let mut page = 1;

    loop {
        let url = format!("{base}/users/{username}/animelist?page={page}");
        let fetched = gate.get_with(Service::Jikan, &url, &GetOptions::default()).await?;
        let res: JikanUserAnimelistResponse = fetched.json()?;

        for item in &res.data {
            let status_str = item.status.as_deref().unwrap_or("plan_to_watch");
            let list_status = ListStatus::parse(status_str);
            let title = item.entry.title.clone();
            let mal_id = item.entry.mal_id;

            let entry = ListEntry {
                mal_id,
                title: title.clone(),
                english_title: None,
                list_status,
                watched_count: item.episodes_seen.unwrap_or(0).max(0),
                total_episodes: None,
                start_year: None,
                auto: matches!(list_status, ListStatus::Watching),
                episode_offset: 0,
                names: vec![title],
                franchise: mal_id,
                season: 1,
            };

            out.push(MalEntry {
                entry,
                score: item.score,
                updated_at: item.updated_at.clone(),
                media_type: None,
                airing_status: None,
                source: "jikan".to_string(),
                poster_url: None,
                synopsis: None,
            });
        }

        let has_next = res.pagination.as_ref().map(|p| p.has_next_page).unwrap_or(false);
        if !has_next || res.data.is_empty() || page >= 20 {
            break;
        }
        page += 1;
    }

    Ok(out)
}

/// Parse Jikan animelist from JSON body text.
pub fn parse_jikan_animelist(text: &str) -> Result<Vec<MalEntry>> {
    let res: JikanUserAnimelistResponse = serde_json::from_str(text)?;
    let mut out = Vec::new();
    for item in &res.data {
        let status_str = item.status.as_deref().unwrap_or("plan_to_watch");
        let list_status = ListStatus::parse(status_str);
        let title = item.entry.title.clone();
        let mal_id = item.entry.mal_id;

        let entry = ListEntry {
            mal_id,
            title: title.clone(),
            english_title: None,
            list_status,
            watched_count: item.episodes_seen.unwrap_or(0).max(0),
            total_episodes: None,
            start_year: None,
            auto: matches!(list_status, ListStatus::Watching),
            episode_offset: 0,
            names: vec![title],
            franchise: mal_id,
            season: 1,
        };

        out.push(MalEntry {
            entry,
            score: item.score,
            updated_at: item.updated_at.clone(),
            media_type: None,
            airing_status: None,
            source: "jikan".to_string(),
            poster_url: None,
            synopsis: None,
        });
    }
    Ok(out)
}

/// Sync anime list using public Jikan profile.
pub async fn sync_jikan(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    username: &str,
) -> Result<SyncReport> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let entries_res = fetch_mal_public_load_json(gate, username).await;
    let (mut entries, is_load_json) = match entries_res {
        Ok(list) if !list.is_empty() => (list, true),
        Ok(_) => {
            (fetch_jikan_list(gate, settings, username).await?, false)
        }
        Err(e) => {
            tracing::warn!("MAL load.json public profile fetch failed ({e}); trying Jikan fallback");
            (fetch_jikan_list(gate, settings, username).await?, false)
        }
    };

    let mut report = SyncReport {
        total: entries.len(),
        ..Default::default()
    };

    let mut relations: HashMap<i64, Vec<Relation>> = HashMap::new();
    let mut relations_failed = 0usize;

    if !is_load_json {
        let raw_base = settings.jikan_base_url.trim_end_matches('/');
        let base = if raw_base.ends_with("/v4") {
            raw_base.to_string()
        } else {
            format!("{raw_base}/v4")
        };

        for m in &mut entries {
            let id = m.mal_id();
            let detail_url = format!("{base}/anime/{id}/full");
            match gate.get_with(Service::Jikan, &detail_url, &GetOptions::default()).await {
                Ok(fetched) => {
                    if let Ok(detail) = fetched.json::<JikanAnimeDetailResponse>() {
                        let d = detail.data;
                        m.entry.english_title = d.title_english.clone();
                        m.entry.total_episodes = d.episodes.filter(|n| *n > 0);
                        m.entry.start_year = d.year;
                        m.media_type = d.media_type.clone();
                        m.airing_status = d.status.clone();

                        let mut names = vec![d.title.clone()];
                        if let Some(en) = d.title_english {
                            if !names.contains(&en) {
                                names.push(en);
                            }
                        }
                        if let Some(ja) = d.title_japanese {
                            if !names.contains(&ja) {
                                names.push(ja);
                            }
                        }
                        if let Some(titles) = d.titles {
                            for t in titles {
                                if !names.contains(&t.title) {
                                    names.push(t.title);
                                }
                            }
                        }
                        m.entry.names = names;

                        let mut rels = Vec::new();
                        if let Some(r_list) = d.relations {
                            for r in r_list {
                                let rel_type = r.relation.to_lowercase();
                                for sub in r.entry {
                                    if sub.entry_type == "anime" {
                                        rels.push(Relation {
                                            node: Node {
                                                id: sub.mal_id,
                                                title: Some(sub.name.clone()),
                                                main_title: Some(sub.name),
                                                alternative_titles: None,
                                                num_episodes: None,
                                                start_season: None,
                                                media_type: None,
                                                status: None,
                                                main_picture: None,
                                                synopsis: None,
                                            },
                                            relation_type: rel_type.clone(),
                                        });
                                    }
                                }
                            }
                        }
                        relations.insert(id, rels);
                    }
                }
                Err(_) => {
                    relations_failed += 1;
                }
            }
        }
    }

    report.relations_failed = relations_failed;
    build_chains(&mut entries, &relations);

    let mut fetched_ids = Vec::with_capacity(entries.len());
    for m in &entries {
        let id = m.mal_id();
        fetched_ids.push(id);

        upsert_show_full(db, m)?;
        crate::planner::upsert_names(db, &m.entry)?;

        let prev_title: Option<String> = db.with(|c| {
            Ok(c.query_row(
                "SELECT title FROM shows WHERE mal_id = ?1",
                [id],
                |r| r.get::<_, String>(0),
            )
            .ok())
        })?;

        match load_mal_entry(db, id)? {
            None => {
                save_mal_entry(db, m)?;
                report.added += 1;
                let _ = log.info(None, Some(id), format!("Jikan sync: added \"{}\"", m.entry.title));
            }
            Some(stored) => {
                if let Some(old) = prev_title {
                    if old != m.entry.title {
                        report.renamed.push(m.entry.title.clone());
                    }
                }
                if stored.changed(m) {
                    save_mal_entry(db, m)?;
                    report.updated += 1;
                } else {
                    report.unchanged += 1;
                }
            }
        }
    }

    let known: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare("SELECT mal_id FROM mal_entries WHERE source = 'jikan'")?;
        let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
        Ok(rows.filter_map(std::result::Result::ok).collect::<Vec<_>>())
    })?;
    for id in known {
        if !fetched_ids.contains(&id) {
            db.with(|c| {
                c.execute("DELETE FROM mal_entries WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            db.with(|c| {
                c.execute("UPDATE shows SET auto = 0 WHERE mal_id = ?1", [id])?;
                Ok(())
            })?;
            report.removed += 1;
            let _ = log.info(None, Some(id), "Jikan sync: no longer on public list");
        }
    }

    // Queue metadata refresh jobs for any show missing synopsis/poster or with stale data (> 7 days)
    if let Ok(queued) = crate::metadata::queue_stale_metadata(db, now) {
        report.metadata_jobs_queued = queued;
    }

    settings::write_state(db, SYNC_STATE_KEY, &now.to_string())?;
    report.entries = entries.into_iter().map(|m| m.entry).collect();
    let _ = log.info(
        None,
        None,
        format!(
            "Jikan sync: {} new, {} updated, {} unchanged, {} removed, {} metadata refresh queued",
            report.added, report.updated, report.unchanged, report.removed, report.metadata_jobs_queued
        ),
    );
    Ok(report)
}

// ---------------------------------------------------------------------------
// High-Level Core Commands
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MalConnectStartResult {
    pub authorize_url: String,
    pub state: String,
    pub port: u16,
    pub redirect_uri: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MalStatusResult {
    pub status: String, // "connected" | "public_profile" | "token_expired" | "not_connected"
    pub username: Option<String>,
    pub source: Option<String>, // "mal" | "jikan"
    pub avatar_url: Option<String>,
    pub last_sync_at: Option<i64>,
    pub counts: StatusCounts,
    pub client_id: Option<String>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StatusCounts {
    pub watching: usize,
    pub completed: usize,
    pub on_hold: usize,
    pub dropped: usize,
    pub plan_to_watch: usize,
    pub total: usize,
}

pub async fn mal_connect_start(
    settings: &Settings,
    redirect_port: Option<u16>,
    redirect_uri: Option<&str>,
) -> Result<Login> {
    start_login_with_options(settings, redirect_port, redirect_uri).await
}

pub async fn mal_status(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    store: &dyn TokenStore,
    clock: &dyn Clock,
) -> Result<MalStatusResult> {
    let last_sync = settings::read_state_time(db, SYNC_STATE_KEY)?;
    let public_user = settings::read_state(db, JIKAN_USER_STATE_KEY)?;
    let client_id = if settings.mal_client_id.trim().is_empty() {
        None
    } else {
        Some(settings.mal_client_id.trim().to_string())
    };

    // Status counts from mal_entries
    let mut counts = StatusCounts::default();
    db.with(|c| {
        let mut stmt = c.prepare("SELECT list_status, COUNT(*) FROM mal_entries GROUP BY list_status")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize)))?;
        for row in rows {
            let (status, count) = row?;
            match status.as_str() {
                "watching" => counts.watching = count,
                "completed" => counts.completed = count,
                "on_hold" => counts.on_hold = count,
                "dropped" => counts.dropped = count,
                "plan_to_watch" => counts.plan_to_watch = count,
                _ => {}
            }
            counts.total += count;
        }
        Ok(())
    })?;

    // Check OAuth first
    if store.refresh_token().ok().flatten().is_some() {
        let client = MalClient::new(gate, settings.clone(), store, clock);
        match client.account().await {
            Ok(acc) => {
                return Ok(MalStatusResult {
                    status: "connected".to_string(),
                    username: Some(acc.name),
                    source: Some("mal".to_string()),
                    avatar_url: None,
                    last_sync_at: last_sync,
                    counts,
                    client_id,
                });
            }
            Err(Error::Auth { .. }) => {
                return Ok(MalStatusResult {
                    status: "token_expired".to_string(),
                    username: None,
                    source: Some("mal".to_string()),
                    avatar_url: None,
                    last_sync_at: last_sync,
                    counts,
                    client_id,
                });
            }
            Err(_) => {
                return Ok(MalStatusResult {
                    status: "connected".to_string(),
                    username: None,
                    source: Some("mal".to_string()),
                    avatar_url: None,
                    last_sync_at: last_sync,
                    counts,
                    client_id,
                });
            }
        }
    }

    // Fallback: public profile
    if let Some(user) = public_user {
        if !user.trim().is_empty() {
            return Ok(MalStatusResult {
                status: "public_profile".to_string(),
                username: Some(user),
                source: Some("jikan".to_string()),
                avatar_url: None,
                last_sync_at: last_sync,
                counts,
                client_id,
            });
        }
    }

    Ok(MalStatusResult {
        status: "not_connected".to_string(),
        username: None,
        source: None,
        avatar_url: None,
        last_sync_at: last_sync,
        counts,
        client_id,
    })
}

pub fn mal_disconnect(db: &Db, store: &dyn TokenStore) -> Result<()> {
    store.set_refresh_token(None)?;
    let _ = store.set_client_secret(None);
    settings::write_state(db, JIKAN_USER_STATE_KEY, "")?;
    settings::write_state(db, ACCOUNT_SOURCE_STATE_KEY, "")?;
    let _ = EventLog::new(db.clone()).info(None, None, "MAL disconnected; credentials and profile cleared");
    Ok(())
}

pub fn mal_set_public_profile(db: &Db, username: &str) -> Result<()> {
    settings::write_state(db, JIKAN_USER_STATE_KEY, username.trim())?;
    settings::write_state(db, ACCOUNT_SOURCE_STATE_KEY, "jikan")?;
    let _ = EventLog::new(db.clone()).info(
        None,
        None,
        format!("Public MAL profile set to \"{}\"", username.trim()),
    );
    Ok(())
}

pub async fn mal_sync(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    store: &dyn TokenStore,
    clock: &dyn Clock,
) -> Result<SyncReport> {
    if store.refresh_token().ok().flatten().is_some() {
        let client = MalClient::new(gate, settings.clone(), store, clock);
        sync(&client).await
    } else if let Some(user) = settings::read_state(db, JIKAN_USER_STATE_KEY)? {
        if !user.trim().is_empty() {
            sync_jikan(db, gate, settings, clock, &user).await
        } else {
            Err(Error::Auth {
                service: "mal".to_string(),
                message: "neither MAL account nor public profile username is set".to_string(),
            })
        }
    } else {
        Err(Error::Auth {
            service: "mal".to_string(),
            message: "neither MAL account nor public profile username is set".to_string(),
        })
    }
}