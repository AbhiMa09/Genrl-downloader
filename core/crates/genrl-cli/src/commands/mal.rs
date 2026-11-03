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
        /// Store the refresh token in memory for this session only (no system keyring).
        #[arg(long)]
        session_only: bool,
    },
    /// Start OAuth login with an optional port and return the auth URL.
    Connect {
        /// Optional client_id to save in settings before starting.
        #[arg(long)]
        client_id: Option<String>,
        /// Optional client_secret for Web applications.
        #[arg(long)]
        client_secret: Option<String>,
        /// Explicit port to bind (default: 21445, the redirect registered on MAL).
        #[arg(long)]
        port: Option<u16>,
        /// Explicit redirect URI (e.g. http://localhost:21445/callback).
        #[arg(long)]
        redirect_uri: Option<String>,
        /// Don't try to open a browser tab automatically.
        #[arg(long)]
        no_open: bool,
        /// Store the refresh token in memory for this session only (no system keyring).
        #[arg(long)]
        session_only: bool,
        /// Output result as JSON.
        #[arg(long)]
        json: bool,
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
    /// Live progress of a running sync or metadata pass, as JSON.
    Progress,
}

#[allow(clippy::too_many_arguments)]
pub async fn mal_login_cmd(
    data_dir: &Path,
    client_id: Option<String>,
    client_secret: Option<String>,
    port: Option<u16>,
    redirect_uri: Option<String>,
    no_open: bool,
    session_only: bool,
    json: bool,
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
    if !session_only && !genrl_core::secrets::is_available() {
        bail!("System keyring is unavailable (no Secret Service / desktop keyring detected). Use --session-only to keep the refresh token in memory for this session, or provide GENRL_MAL_REFRESH_TOKEN.");
    }
    let memory_store;
    let keyring_store = KeyringStore;
    let store: &dyn TokenStore = if session_only {
        memory_store = genrl_core::mal::MemoryStore::new();
        &memory_store
    } else {
        &keyring_store
    };
    let client_secret = client_secret.or_else(|| std::env::var("GENRL_MAL_CLIENT_SECRET").ok());
    if let Some(sec) = client_secret {
        let sec = sec.trim();
        if !sec.is_empty() {
            store.set_client_secret(Some(sec))?;
        }
    }
    let gate = Gate::new(db.clone())?;
    let clock = SystemClock;
    let login = mal::mal_connect_start(&settings, port, redirect_uri.as_deref()).await?;
    if !json {
        println!("Authorize URL:\n\n  {}\n", login.url());
        println!("Listening on: {}", login.redirect_uri());
    }
    let _ = std::fs::write(data_dir.join("mal_authorize_url.txt"), login.url());
    if !no_open {
        open_system_browser(login.url())?;
    }
    if !json {
        println!("waiting for the redirect on 127.0.0.1 (up to 5 minutes)...");
    }
    let client = MalClient::new(&gate, settings, store, &clock);
    let finish_res = login
        .finish(&client, std::time::Duration::from_secs(300))
        .await;
    let _ = std::fs::remove_file(data_dir.join("mal_authorize_url.txt"));
    finish_res?;
    // Credentials are stored now; do not leave the account in public-profile mode
    // if the subsequent profile lookup has a transient network failure.
    settings::write_state(&db, mal::ACCOUNT_SOURCE_STATE_KEY, "mal")?;
    let account = client.account().await?;
    settings::write_state(&db, "mal_account", &account.name)?;

    if json {
        let token_val = if session_only {
            store.refresh_token()?.unwrap_or_default()
        } else {
            String::new()
        };
        let mut map = serde_json::Map::new();
        map.insert("status".into(), "connected".into());
        map.insert("username".into(), account.name.clone().into());
        map.insert("user_id".into(), account.id.into());
        map.insert("session_only".into(), session_only.into());
        if session_only {
            map.insert("session_token".into(), token_val.into());
        }
        println!("{}", serde_json::to_string(&map)?);
    } else if session_only {
        println!(
            "connected as {} (id {}) [session-only mode: refresh token held in memory]",
            account.name, account.id
        );
    } else {
        println!("connected as {} (id {})", account.name, account.id);
    }
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
    let _lock = match mal::try_lock_mal_sync(data_dir)? {
        Some(l) => l,
        None => {
            println!(
                "{}",
                serde_json::json!({
                    "status": "already_running",
                    "message": "MAL sync already in progress"
                })
            );
            return Ok(());
        }
    };
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db.clone())?;
    let store = KeyringStore;
    let clock = SystemClock;
    let report = mal::mal_sync(&db, &gate, &settings, &store, &clock).await?;
    println!("{}", report.summary());
    Ok(())
}

pub fn mal_progress_cmd(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    println!("{}", genrl_core::progress::read_all(&db));
    Ok(())
}

fn browser_command(url: &str, windows: bool, wsl: bool, macos: bool) -> std::process::Command {
    if windows || wsl {
        let script = format!("Start-Process '{}'", url.replace('\'', "''"));
        let mut command = std::process::Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        command
    } else {
        let mut command = std::process::Command::new(if macos { "open" } else { "xdg-open" });
        command.arg(url);
        command
    }
}

fn open_system_browser(url: &str) -> Result<()> {
    let wsl = std::path::Path::new("/proc/sys/fs/binfmt_misc/WSLInterop").exists()
        || std::env::var_os("WSL_DISTRO_NAME").is_some();
    let mut command = browser_command(url, cfg!(windows), wsl, cfg!(target_os = "macos"));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW for the helper process.
    }
    let status = command.status().map_err(|_| anyhow::anyhow!(
        "Could not open the default browser. Check the default HTTPS browser in system settings and retry."
    ))?;
    if !status.success() {
        bail!("Could not open the default browser. Check the default HTTPS browser in system settings and retry.");
    }
    Ok(())
}

#[cfg(test)]
mod browser_tests {
    use super::browser_command;

    #[test]
    fn windows_and_wsl_use_default_browser_without_shell_interpolation() {
        let url = "https://example.invalid/authorize?state=fixture&code_challenge=fixture";
        for (windows, wsl) in [(true, false), (false, true), (true, true)] {
            let command = browser_command(url, windows, wsl, false);
            assert_eq!(command.get_program(), "powershell.exe");
            let args: Vec<_> = command
                .get_args()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            assert_eq!(args[0], "-NoProfile");
            assert_eq!(args[1], "-NonInteractive");
            assert_eq!(args[2], "-Command");
            assert!(args[3].contains(url));
        }
    }

    #[test]
    fn unix_launchers_keep_url_as_one_argument() {
        for (macos, program) in [(true, "open"), (false, "xdg-open")] {
            let command = browser_command("https://example.invalid/?a=1&b=2", false, false, macos);
            assert_eq!(command.get_program(), program);
            assert_eq!(command.get_args().count(), 1);
        }
    }
}
