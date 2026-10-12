//! The qBittorrent WebUI client.
//!
//! Everything the app does with qBittorrent goes through here, and every call
//! itself goes through the shared [`Gate`] (`Service::Qbit`), so the limiter,
//! the circuit breaker and the backoff apply to it like any other service.
//!
//! Three design requirements shape this module:
//!
//! * **session** — log in once, keep the session cookie on every call
//!   (`QBT_SID_…` in 5.x, `SID` in 4.x), and re-log in automatically when the
//!   API answers 403 (expired session);
//! * **version** — 4.x spelled the controls `pause`/`resume`, 5.x renamed them
//!   `stop`/`start`; we read the version once and hide the difference;
//! * **health** — a launch health check that can start `qbittorrent-nox` when
//!   the settings allow it, and otherwise explains how to fix it.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::http::{Gate, GetOptions, RetryPolicy, Service};
use crate::secrets::{self, Secret};
use crate::settings::Settings;

/// Which generation of the WebUI API we are talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiGen {
    V4,
    V5,
}

impl ApiGen {
    /// `"v5.2.4"` → V5, `"v4.6.4"` → V4, anything unparseable is V4 (the
    /// pause/resume names are the older, longer-lived spelling).
    pub fn parse(version: &str) -> Self {
        let major = version
            .trim()
            .trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|first| first.parse::<u32>().ok())
            .unwrap_or(4);
        if major >= 5 {
            ApiGen::V5
        } else {
            ApiGen::V4
        }
    }

    /// The control path for starting (`true`) or stopping (`false`) torrents.
    pub fn control_path(self, want_running: bool) -> &'static str {
        match (self, want_running) {
            (ApiGen::V4, true) => "resume",
            (ApiGen::V4, false) => "pause",
            (ApiGen::V5, true) => "start",
            (ApiGen::V5, false) => "stop",
        }
    }
}

/// What the health check reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// Version string as qBittorrent reports it, e.g. `v5.2.4`.
    pub version: String,
    pub gen: ApiGen,
}

/// One row of `/api/v2/torrents/info`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    /// `0.0 ..= 1.0`.
    pub progress: f64,
    /// Bytes per second, down and up.
    pub dlspeed: i64,
    pub upspeed: i64,
    /// Seconds; qBittorrent reports `8640000` for "unknown".
    pub eta: i64,
    #[serde(default)]
    pub num_seeds: i64,
    #[serde(default)]
    pub num_leechs: i64,
    /// Total size in bytes.
    pub size: i64,
    /// `downloading`, `metaDL`, `stalledDL`, `stoppedDL` (5.x) or
    /// `pausedDL` (4.x), `uploading`, `error`, …
    pub state: String,
    #[serde(default)]
    pub save_path: String,
    #[serde(default)]
    pub content_path: String,
    #[serde(default)]
    pub amount_left: i64,
    #[serde(default)]
    pub added_on: i64,
    #[serde(default)]
    pub completion_on: i64,
    #[serde(default)]
    pub category: String,
}

impl Torrent {
    /// Finished downloading (every byte is on disk).
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Blocked or errored rather than merely waiting for peers.
    pub fn is_in_trouble(&self) -> bool {
        self.state.contains("error") || self.state.contains("missingFiles")
    }

    /// Queued, stopped or paused: nothing is moving right now.
    pub fn is_stopped(&self) -> bool {
        self.state.contains("stopped") || self.state.contains("paused") || self.state.contains("queued")
    }
}

/// Total transfer speeds for the status strip.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferSpeeds {
    #[serde(rename = "dl_info_speed")]
    pub download: i64,
    #[serde(rename = "up_info_speed")]
    pub upload: i64,
}

pub struct Qbit {
    base: String,
    username: String,
    /// `None` only when nothing is stored; an empty password then tries the
    /// WebUI's "bypass authentication for localhost" setup.
    password: Option<String>,
    /// The full session cookie pair exactly as `Set-Cookie` handed it out:
    /// `QBT_SID_8080=…` (5.x) or `SID=…` (4.x).
    session: std::sync::Mutex<Option<String>>,
    gen: std::sync::Mutex<Option<ApiGen>>,
}

impl Qbit {
    /// Build from settings, reading the password out of the keyring (or the
    /// `GENRL_QBIT_PASSWORD` override).
    pub fn from_settings(settings: &Settings) -> Result<Self> {
        let password = secrets::get(Secret::QbitPassword)?;
        Ok(Self::new(
            &settings.qbit_url,
            &settings.qbit_username,
            password,
        ))
    }

    pub fn new(base: &str, username: &str, password: Option<String>) -> Self {
        Qbit {
            base: base.trim_end_matches('/').to_string(),
            username: username.to_string(),
            password,
            session: std::sync::Mutex::new(None),
            gen: std::sync::Mutex::new(None),
        }
    }

    /// Base URL of the WebUI.
    pub fn base(&self) -> &str {
        &self.base
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    /// qBittorrent is local and idempotent-friendly: one short retry rides out
    /// a restart race, and anything longer belongs to the job queue. The
    /// circuit breaker (five failures, fifteen minutes) is what parks jobs in
    /// `blocked`, not retries inside one call.
    fn quick_retry() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 2,
            schedule: vec![Duration::from_secs(2)],
            max_total_wait: Duration::from_secs(5),
            max_delay: Duration::from_secs(5),
        }
    }

    fn session(&self) -> Option<String> {
        self.session.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn store_session(&self, cookie: Option<String>) {
        *self.session.lock().unwrap_or_else(|e| e.into_inner()) = cookie;
    }

    /// Request options: the session cookie plus the short retry.
    fn opts(&self, session: Option<&str>) -> GetOptions {
        let mut headers = Vec::new();
        if let Some(cookie) = session {
            headers.push(("Cookie".to_string(), cookie.to_string()));
        }
        GetOptions {
            headers,
            retry: Some(Self::quick_retry()),
            ..Default::default()
        }
    }

    /// GET an API path, re-logging in once when the session has expired.
    async fn get(&self, gate: &Gate, path: &str) -> Result<crate::http::Fetched> {
        let url = self.url(path);
        let opts = self.opts(self.session().as_deref());
        match gate.get_with(Service::Qbit, &url, &opts).await {
            Err(Error::Status { status: 403, .. }) => {
                self.login(gate).await?;
                let fresh = self.opts(self.session().as_deref());
                gate.get_with(Service::Qbit, &url, &fresh).await
            }
            other => other,
        }
    }

    /// POST a form to an API path, re-logging in once on a 403.
    async fn post(
        &self,
        gate: &Gate,
        path: &str,
        form: &[(String, String)],
    ) -> Result<crate::http::Fetched> {
        let url = self.url(path);
        match gate
            .post_form(Service::Qbit, &url, form, &self.opts(self.session().as_deref()))
            .await
        {
            Err(Error::Status { status: 403, .. }) => {
                self.login(gate).await?;
                gate.post_form(
                    Service::Qbit,
                    &url,
                    form,
                    &self.opts(self.session().as_deref()),
                )
                .await
            }
            other => other,
        }
    }

    /// The credential failure the design asks for: names the service, never
    /// the secret, and says exactly how to fix it.
    fn auth_error(&self, detail: &str) -> Error {
        Error::Auth {
            service: "qBittorrent".to_string(),
            message: format!(
                "{detail}; set the WebUI password with `genrl qbit password` or {}",
                Secret::QbitPassword.env_var()
            ),
        }
    }

    /// Log in and remember the session cookie. Called lazily: the first API
    /// call gets a 403, this runs, and the call is retried once.
    ///
    /// The two WebUI dialects both work: 4.x answers `200 Ok.` with a `SID=…`
    /// cookie, 5.2 answers `204` with an empty body and a port-suffixed
    /// `QBT_SID_…` cookie. The cookie is always sent back exactly as it was
    /// handed out. A `400`/`401` status or a body that says `Fails.` becomes
    /// an [`Error::Auth`] carrying the fix.
    async fn login(&self, gate: &Gate) -> Result<()> {
        let url = self.url("/api/v2/auth/login");
        let form = [
            ("username".to_string(), self.username.clone()),
            (
                "password".to_string(),
                self.password.clone().unwrap_or_default(),
            ),
        ];
        let fetched = match gate
            .post_form(
                Service::Qbit,
                &url,
                &form,
                &GetOptions {
                    retry: Some(Self::quick_retry()),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(fetched) => fetched,
            // 4.x refuses with `400 Fails.`, 5.x with `401 Unauthorized`.
            Err(Error::Status {
                status: status @ (400 | 401 | 403),
                ..
            }) => return Err(self.auth_error(&format!("login refused with HTTP {status}"))),
            Err(err) => return Err(err),
        };

        // Take the cookie pair (`QBT_SID_8080=…` / `SID=…`) before reading
        // the body, then judge the reply: empty body (5.x) or `Ok.` (4.x)
        // against a session cookie means success.
        let cookie = fetched
            .set_cookie
            .as_deref()
            .and_then(|header| header.split(';').next())
            .map(str::trim)
            .filter(|pair| pair.contains('='))
            .map(str::to_string);
        let body = fetched.text()?;
        let body = body.trim();

        if !body.is_empty() && !body.eq_ignore_ascii_case("Ok.") {
            return Err(self.auth_error(body));
        }
        let cookie = cookie.ok_or_else(|| self.auth_error("the reply carried no session cookie"))?;
        self.store_session(Some(cookie));

        if self.username.eq_ignore_ascii_case("admin") && self.password.as_deref() == Some("adminadmin")
        {
            tracing::warn!(
                "qBittorrent is using its default WebUI credentials; change the password"
            );
        }
        Ok(())
    }

    /// Health check: version plus which API generation to speak.
    pub async fn health(&self, gate: &Gate) -> Result<Health> {
        let fetched = self.get(gate, "/api/v2/app/version").await?;
        let version = fetched.text()?.trim().to_string();
        let gen = ApiGen::parse(&version);
        *self.gen.lock().unwrap_or_else(|e| e.into_inner()) = Some(gen);
        Ok(Health { version, gen })
    }

    async fn api_gen(&self, gate: &Gate) -> Result<ApiGen> {
        if let Some(gen) = *self.gen.lock().unwrap_or_else(|e| e.into_inner()) {
            return Ok(gen);
        }
        self.health(gate).await.map(|health| health.gen)
    }

    /// All torrents (optionally filtered: `downloading`, `seeding`, `done`, …).
    pub async fn list(&self, gate: &Gate, filter: Option<&str>) -> Result<Vec<Torrent>> {
        let path = match filter {
            Some(filter) => format!("/api/v2/torrents/info?filter={filter}"),
            None => "/api/v2/torrents/info".to_string(),
        };
        let fetched = self.get(gate, &path).await?;
        Ok(serde_json::from_slice(&fetched.body)?)
    }

    /// One torrent by info hash; `None` when qBittorrent does not know it.
    pub async fn get_torrent(&self, gate: &Gate, hash: &str) -> Result<Option<Torrent>> {
        let path = format!("/api/v2/torrents/info?hashes={}", hash.to_uppercase());
        let fetched = self.get(gate, &path).await?;
        let mut torrents: Vec<Torrent> = serde_json::from_slice(&fetched.body)?;
        Ok(torrents.pop())
    }

    /// Add torrents by magnet or `.torrent` URL (newline-separated, as the
    /// API expects). Optional save path and category.
    pub async fn add(
        &self,
        gate: &Gate,
        urls: &[String],
        save_path: Option<&str>,
        category: Option<&str>,
    ) -> Result<()> {
        let mut form = vec![("urls".to_string(), urls.join("\n"))];
        if let Some(save_path) = save_path {
            form.push(("savepath".to_string(), save_path.to_string()));
        }
        if let Some(category) = category {
            form.push(("category".to_string(), category.to_string()));
        }
        self.post(gate, "/api/v2/torrents/add", &form).await?;
        Ok(())
    }

    /// Start or stop a torrent (4.x pause/resume vs 5.x stop/start, hidden).
    pub async fn set_active(&self, gate: &Gate, hash: &str, active: bool) -> Result<()> {
        let gen = self.api_gen(gate).await?;
        let path = format!("/api/v2/torrents/{}", gen.control_path(active));
        let form = [("hashes".to_string(), hash.to_uppercase())];
        self.post(gate, &path, &form).await?;
        Ok(())
    }

    /// Remove a torrent, optionally deleting its data.
    pub async fn remove(&self, gate: &Gate, hash: &str, delete_files: bool) -> Result<()> {
        let form = [
            ("hashes".to_string(), hash.to_uppercase()),
            (
                "deleteFiles".to_string(),
                if delete_files { "true" } else { "false" }.to_string(),
            ),
        ];
        self.post(gate, "/api/v2/torrents/delete", &form).await?;
        Ok(())
    }

    /// Total transfer speeds for the status strip.
    pub async fn speeds(&self, gate: &Gate) -> Result<TransferSpeeds> {
        let fetched = self.get(gate, "/api/v2/transfer/info").await?;
        Ok(serde_json::from_slice(&fetched.body)?)
    }
}

/// The design's launch sequence: health check first, start `qbittorrent-nox`
/// when the setting allows it, and otherwise explain exactly what to do.
pub async fn ensure_running(gate: &Gate, settings: &Settings) -> Result<Health> {
    let qbit = Qbit::from_settings(settings)?;
    match qbit.health(gate).await {
        Ok(health) => Ok(health),
        Err(err) if err.is_connection_failure() && settings.qbit_start_if_installed => {
            start_daemon()?;
            wait_until_listening(settings).await?;
            match qbit.health(gate).await {
                Ok(health) => Ok(health),
                Err(_) => Err(setup_help(settings)),
            }
        }
        Err(err) if err.is_connection_failure() => Err(setup_help(settings)),
        Err(err) => Err(err),
    }
}

/// Nothing is listening where the WebUI should be: the design's "clear setup
/// instructions if WebUI is disabled". Public so the CLI's health check can
/// show it instead of a raw connection error.
pub fn setup_help(settings: &Settings) -> Error {
    Error::Other(format!(
        "no qBittorrent WebUI at {} — start qBittorrent (or install qbittorrent-nox) and \
         enable Settings → WebUI on that port, then set the password with `genrl qbit password`",
        settings.qbit_url
    ))
}

/// Start `qbittorrent-nox --daemon` when it is installed. The GUI binary is
/// deliberately not started: a desktop window is the user's call to make.
fn start_daemon() -> Result<()> {
    use std::process::{Command, Stdio};
    match Command::new("qbittorrent-nox")
        .arg("--daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => {
            tracing::info!("started qbittorrent-nox --daemon");
            Ok(())
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(Error::Other(
            "qbittorrent-nox is not installed; install it (recommended) or start qBittorrent \
             with its WebUI enabled"
                .to_string(),
        )),
        Err(err) => Err(Error::Io(err)),
    }
}

/// Probe the WebUI port directly until it accepts a connection (up to ~20 s).
///
/// A plain TCP connect — not an HTTP request through the gate — so waiting for
/// our own daemon never counts against the circuit breaker.
async fn wait_until_listening(settings: &Settings) -> Result<()> {
    let url = reqwest::Url::parse(&settings.qbit_url)
        .map_err(|_| Error::InvalidUrl(settings.qbit_url.clone()))?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::InvalidUrl(settings.qbit_url.clone()))?
        .to_string();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| Error::InvalidUrl(settings.qbit_url.clone()))?;

    for _ in 0..40 {
        match tokio::net::TcpStream::connect((host.as_str(), port)).await {
            Ok(_) => return Ok(()),
            Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
        }
    }
    Err(setup_help(settings))
}

/// Extract the 40-hex info hash from a magnet link, if it carries one.
pub fn magnet_info_hash(link: &str) -> Option<String> {
    let rest = link.strip_prefix("magnet:?").or_else(|| link.strip_prefix("magnet:"))?;
    for pair in rest.trim_start_matches('?').split('&') {
        if let Some(value) = pair.strip_prefix("xt=urn:btih:") {
            let hash = value.to_lowercase();
            if hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(hash);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_generation_from_version_strings() {
        assert_eq!(ApiGen::parse("v5.2.4"), ApiGen::V5);
        assert_eq!(ApiGen::parse("v4.6.7"), ApiGen::V4);
        assert_eq!(ApiGen::parse("5.0.0"), ApiGen::V5);
        assert_eq!(ApiGen::parse(""), ApiGen::V4, "unknown falls back");
    }

    #[test]
    fn control_paths_follow_the_version() {
        assert_eq!(ApiGen::V4.control_path(true), "resume");
        assert_eq!(ApiGen::V4.control_path(false), "pause");
        assert_eq!(ApiGen::V5.control_path(true), "start");
        assert_eq!(ApiGen::V5.control_path(false), "stop");
    }

    #[test]
    fn magnets_yield_their_info_hash() {
        let hash = "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b";
        let magnet = format!("magnet:?xt=urn:btih:{hash}&dn=Show&tr=udp://tracker.example");
        assert_eq!(magnet_info_hash(&magnet).as_deref(), Some(hash));
        assert_eq!(
            magnet_info_hash("magnet:?dn=NoHash").as_deref(),
            None,
            "no xt, no hash"
        );
        assert_eq!(
            magnet_info_hash("magnet:?xt=urn:btih:SHORT").as_deref(),
            None,
            "must be a full sha1"
        );
        assert_eq!(magnet_info_hash("https://example.com"), None);
    }

    #[test]
    fn torrent_states_are_classified() {
        let stopped = Torrent {
            hash: "h".into(),
            name: "n".into(),
            progress: 0.5,
            dlspeed: 0,
            upspeed: 0,
            eta: 0,
            num_seeds: 0,
            num_leechs: 0,
            size: 1,
            state: "stoppedDL".into(),
            save_path: String::new(),
            content_path: String::new(),
            amount_left: 1,
            added_on: 0,
            completion_on: 0,
            category: String::new(),
        };
        assert!(stopped.is_stopped());
        assert!(!stopped.is_complete());
        assert!(!stopped.is_in_trouble());

        let done = Torrent {
            state: "uploading".into(),
            progress: 1.0,
            ..stopped.clone()
        };
        assert!(done.is_complete());
        assert!(!done.is_stopped());

        let broken = Torrent {
            state: "error".into(),
            progress: 0.1,
            ..stopped
        };
        assert!(broken.is_in_trouble());
    }
}
