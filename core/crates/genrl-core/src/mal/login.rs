use std::time::Duration;

use crate::error::{Error, Result};
use crate::settings::Settings;

use super::client::MalClient;
use super::pkce::{authorize_url, code_challenge, pct_decode, pkce_verifier, random_token};

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
pub async fn start_login_with_port(
    settings: &Settings,
    override_port: Option<u16>,
) -> Result<Login> {
    start_login_with_options(settings, override_port, None).await
}

/// Start login with an optional explicit port and optional redirect URI override.
pub async fn start_login_with_options(
    settings: &Settings,
    override_port: Option<u16>,
    override_redirect_uri: Option<&str>,
) -> Result<Login> {
    let client_id = settings.mal_client_id.trim().to_string();
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
    let url = authorize_url(
        &client_id,
        &redirect_uri,
        &state,
        &code_challenge(&verifier),
    );
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
pub(crate) fn parse_query(query: &str) -> Vec<(String, String)> {
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
            let _ = stream
                .write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n")
                .await;
            let _ = stream.shutdown().await;
            continue;
        }

        let query = target.split_once('?').map(|(_, q)| q).unwrap_or("");
        let params = parse_query(query);

        let code = params
            .iter()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.clone());
        let state = params
            .iter()
            .find(|(k, _)| k == "state")
            .map(|(_, v)| v.clone());
        let error = params
            .iter()
            .find(|(k, _)| k == "error")
            .map(|(_, v)| v.clone());

        let ok = code.is_some() && state.as_deref() == Some(expected_state) && error.is_none();
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
            (_, _, Some(denied)) => {
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: format!("MyAnimeList denied the login: {denied}"),
                })
            }
            (Some(code), Some(state), None) if state == expected_state => return Ok(code),
            _ => {
                return Err(Error::Auth {
                    service: "mal".to_string(),
                    message: "the redirect state did not match; refusing the code".to_string(),
                })
            }
        }
    }
}
