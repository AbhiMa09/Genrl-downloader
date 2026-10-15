use std::time::Duration;

use crate::error::{Error, Result};
use crate::http::Gate;
use crate::settings::Settings;

use super::client::Qbit;
use super::types::Health;

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
