use std::time::Duration;

use crate::error::{Error, Result};
use crate::http::{Gate, GetOptions, RetryPolicy, Service};
use crate::secrets::{self, Secret};
use crate::settings::Settings;

use super::types::{ApiGen, Health, Torrent, TransferSpeeds};

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
        self.session
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
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
            .post_form(
                Service::Qbit,
                &url,
                form,
                &self.opts(self.session().as_deref()),
            )
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
        let cookie =
            cookie.ok_or_else(|| self.auth_error("the reply carried no session cookie"))?;
        self.store_session(Some(cookie));

        if self.username.eq_ignore_ascii_case("admin")
            && self.password.as_deref() == Some("adminadmin")
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
