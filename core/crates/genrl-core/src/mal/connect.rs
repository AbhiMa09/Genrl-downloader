use crate::clock::Clock;
use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::EventLog;
use crate::http::Gate;
use crate::settings::{self, Settings};

use super::client::MalClient;
use super::jikan_parse::{ACCOUNT_SOURCE_STATE_KEY, JIKAN_USER_STATE_KEY};
use super::jikan_sync::sync_jikan;
use super::login::{start_login_with_options, Login};
use super::pkce::SYNC_STATE_KEY;
use super::sync::{sync, SyncReport};
use super::token::TokenStore;

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
        let mut stmt =
            c.prepare("SELECT list_status, COUNT(*) FROM mal_entries GROUP BY list_status")?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize))
        })?;
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
    let _ = EventLog::new(db.clone()).info(
        None,
        None,
        "MAL disconnected; credentials and profile cleared",
    );
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
