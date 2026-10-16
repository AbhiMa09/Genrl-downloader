use anyhow::{bail, Result};
use clap::Subcommand;
use genrl_core::clock::{Clock, SystemClock};
use genrl_core::http::Gate;
use genrl_core::mal::{self, KeyringStore, MalClient, TokenStore};
use genrl_core::{settings, Settings};
use std::path::Path;

use super::common::open_db;

#[derive(Subcommand, Debug)]
pub enum MalCommand {
    /// Connect to MyAnimeList via OAuth (PKCE) and store the refresh token.
    Login {
        /// Optional client_id to save in settings before starting.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional client_secret for Web applications.
        #[arg(long)]
        client_secret: Option<String>,
        /// Don't try to open a browser tab automatically.
        #[arg(long)]
        no_open: bool,
    },
    /// Start OAuth login with an optional port and return the auth URL.
    Connect {
        /// Optional client_id to save in settings before starting.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional client_secret for Web applications.
        #[arg(long)]
        client_secret: Option<String>,
        /// Explicit port to bind (default: 21445).
        #[arg(long)]
        port: Option<u16>,
        /// Explicit redirect URI (e.g. http://localhost:21445/callback).
        #[arg(long)]
        redirect_uri: Option<String>,
        /// Don't try to open a browser tab automatically.
        #[arg(long)]
        no_open: bool,
    },
    /// Disconnect MAL and delete credentials.
    Disconnect,
    /// Set a public MAL username for fallback Jikan syncing.
    PublicProfile {
        /// Public username on MyAnimeList.
        username: String,
    },
    /// Show whether we are connected, the account, the last sync and list counts.
    Status {
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Fetch the list, diff it against the database, store added/changed/removed shows.
    Sync,
}

pub async fn mal_login_cmd(
    data_dir: &Path,
    client_id: Option<String>,
    client_secret: Option<String>,
    port: Option<u16>,
    redirect_uri: Option<String>,
    no_open: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let mut settings = Settings::load(&db)?;
    if let Some(cid) = client_id {
        let cid = cid.trim().to_string();
        if !cid.is_empty() {
            settings.mal_client_id = cid;
            settings.save(&db)?;
        }
    }
    if settings.mal_client_id.trim().is_empty() {
        bail!("set settings.mal_client_id first (myanimelist.net/apiconfig)");
    }
    let store = KeyringStore;
    if let Some(sec) = client_secret {
        let sec = sec.trim();
        if !sec.is_empty() {
            store.set_client_secret(Some(sec))?;
        }
    }
    let gate = Gate::new(db.clone())?;
    let clock = SystemClock;
    let login = mal::mal_connect_start(&settings, port, redirect_uri.as_deref()).await?;
    println!("Authorize URL:\n\n  {}\n", login.url());
    println!("Listening on: {}", login.redirect_uri());
    if !no_open {
        let _ = std::process::Command::new("xdg-open")
            .arg(login.url())
            .spawn();
    }
    println!("waiting for the redirect on 127.0.0.1 (up to 5 minutes)...");
    let client = MalClient::new(&gate, settings, &store, &clock);
    login
        .finish(&client, std::time::Duration::from_secs(300))
        .await?;
    let account = client.account().await?;
    settings::write_state(&db, "mal_account", &account.name)?;
    settings::write_state(&db, mal::ACCOUNT_SOURCE_STATE_KEY, "mal")?;
    println!("connected as {} (id {})", account.name, account.id);
    Ok(())
}

pub fn mal_disconnect_cmd(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let store = KeyringStore;
    mal::mal_disconnect(&db, &store)?;
    println!("MAL disconnected and credentials cleared.");
    Ok(())
}

pub fn mal_public_profile_cmd(data_dir: &Path, username: &str) -> Result<()> {
    let db = open_db(data_dir)?;
    mal::mal_set_public_profile(&db, username)?;
    println!("Public MAL profile username set to: {username}");
    Ok(())
}

pub async fn mal_status_cmd(data_dir: &Path, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let store = KeyringStore;
    let clock = SystemClock;
    let res = mal::mal_status(&db, &gate, &settings, &store, &clock).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&res)?);
        return Ok(());
    }

    println!("status      : {}", res.status);
    if let Some(user) = res.username {
        println!("username    : {user}");
    }
    if let Some(src) = res.source {
        println!("source      : {src}");
    }
    let client_id = settings.mal_client_id.trim();
    println!(
        "client id   : {}",
        if client_id.is_empty() {
            "(unset)"
        } else {
            client_id
        }
    );
    println!("total anime : {}", res.counts.total);
    println!("  watching  : {}", res.counts.watching);
    println!("  completed : {}", res.counts.completed);
    println!("  on hold   : {}", res.counts.on_hold);
    println!("  dropped   : {}", res.counts.dropped);
    println!("  plan2watch: {}", res.counts.plan_to_watch);
    if let Some(ts) = res.last_sync_at {
        println!("last sync   : {} seconds ago", clock.now() - ts);
    } else {
        println!("last sync   : never");
    }
    Ok(())
}

pub async fn mal_sync_cmd(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let store = KeyringStore;
    let clock = SystemClock;
    let report = mal::mal_sync(&db, &gate, &settings, &store, &clock).await?;
    println!("{}", report.summary());
    Ok(())
}
