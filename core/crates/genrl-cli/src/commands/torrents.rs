use std::path::Path;
use anyhow::{bail, Result};
use clap::Subcommand;
use genrl_core::http::{is_acceptable_link, Gate, Service};
use genrl_core::nyaa::SearchQuery;
use genrl_core::{filename, nyaa, qbit, secrets, EventLog, Settings};

use super::common::{fmt_eta, human_bytes, open_db, truncate_text};

#[derive(Subcommand, Debug)]
pub enum QbitCommand {
    /// Log in, read the version, show transfer speeds.
    Test,
    /// Start qbittorrent-nox when it is missing and the settings allow it.
    Start,
    /// Store the WebUI password in the system keyring.
    Password {
        /// Forget the stored password instead of setting one.
        #[arg(long)]
        delete: bool,
    },
}

pub async fn search_cmd(
    data_dir: &Path,
    query: &str,
    min_seeders: u32,
    limit: usize,
    any_category: bool,
    json: bool,
) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db)?;

    let mut search = SearchQuery::new(query);
    search.min_seeders = min_seeders;
    search.limit = limit.clamp(1, 1000);
    if any_category {
        search = search.without_category();
    }

    let items = nyaa::search(&gate, &settings, &search).await?;
    if items.is_empty() {
        println!("no candidates matched {query:?}");
        return Ok(());
    }

    if json {
        let rows: Vec<serde_json::Value> = items
            .iter()
            .map(|item| {
                let release = filename::parse_release(&item.title);
                serde_json::json!({
                    "title": item.title,
                    "link": item.link,
                    "magnet": item.magnet(),
                    "info_hash": item.info_hash,
                    "seeders": item.seeders,
                    "leechers": item.leechers,
                    "size_bytes": item.size_bytes,
                    "size_text": item.size_text,
                    "trusted": item.trusted,
                    "remake": item.remake,
                    "published_at": item.published_at,
                    "category": item.category,
                    "parsed": {
                        "title": release.title,
                        "group": release.group,
                        "season": release.season,
                        "episode": release.episode,
                        "is_batch": release.is_batch,
                        "resolution": release.resolution,
                        "codec": release.codec,
                        "source": release.source,
                        "dual_audio": release.dual_audio,
                        "crc": release.crc,
                    },
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    println!(
        "{:>3}  {:>4}  {:>7}  {:<12}  {:>4}  {:<5}  {:<50}  {}",
        "#", "SEE", "SIZE", "GROUP", "EP", "RES", "TITLE", "LINK"
    );
    for (index, item) in items.iter().enumerate() {
        let release = filename::parse_release(&item.title);
        let group = release.group.clone().unwrap_or_else(|| "-".into());
        let episode = if let Some(episode) = release.episode {
            format!("{episode}")
        } else if release.is_batch {
            "batch".into()
        } else {
            "-".into()
        };
        let resolution = release.resolution.clone().unwrap_or_else(|| "-".into());
        println!(
            "{:>3}  {:>4}  {:>7}  {:<12}  {:>4}  {:<5}  {:<50}  {}",
            index + 1,
            item.seeders,
            item.size_text,
            truncate_text(&group, 12),
            episode,
            resolution,
            truncate_text(&item.title, 50),
            item.link,
        );
    }
    println!();
    println!("grab one: genrl grab <LINK>");
    Ok(())
}

pub async fn grab(
    data_dir: &Path,
    source: &str,
    to: Option<&str>,
    category: &str,
    timeout: u64,
    no_watch: bool,
) -> Result<()> {
    if !is_acceptable_link(source) {
        bail!("not a magnet link or a public https .torrent URL: {source}");
    }

    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let log = EventLog::new(db.clone());
    let gate = Gate::new(db)?;

    // Health check first: starts qbittorrent-nox when the settings allow it,
    // otherwise explains what to fix.
    let health = qbit::ensure_running(&gate, &settings).await?;
    eprintln!(
        "qBittorrent {} ({:?}) at {}",
        health.version, health.gen, settings.qbit_url
    );

    let client = qbit::Qbit::from_settings(&settings)?;
    let known_hash = source
        .strip_prefix("magnet:")
        .and_then(|_| qbit::magnet_info_hash(source));

    // For a `.torrent` URL the hash only appears once qBittorrent has fetched
    // and parsed the file, so snapshot what exists now and diff later.
    let mut before = std::collections::HashSet::new();
    if known_hash.is_none() {
        for torrent in client.list(&gate, None).await? {
            before.insert(torrent.hash);
        }
    }

    let save_path = to.map(str::to_string).or_else(|| {
        if settings.download_folder.is_empty() {
            None
        } else {
            Some(settings.download_folder.clone())
        }
    });
    client
        .add(
            &gate,
            &[source.to_string()],
            save_path.as_deref(),
            Some(category),
        )
        .await?;
    log.info(None, None, format!("added {source}"))?;
    eprintln!("added {source}");
    if no_watch {
        return Ok(());
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);
    let mut hash = known_hash;
    loop {
        if std::time::Instant::now() >= deadline {
            bail!("gave up after {timeout}s; run `genrl torrents` to see where it stands");
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        if hash.is_none() {
            let current: std::collections::HashSet<String> = client
                .list(&gate, None)
                .await?
                .into_iter()
                .map(|torrent| torrent.hash)
                .collect();
            let fresh: Vec<String> = current
                .into_iter()
                .filter(|hash| !before.contains(hash))
                .collect();
            match fresh.len() {
                0 => {
                    eprintln!("waiting for qBittorrent to fetch the torrent file…");
                    continue;
                }
                1 => {
                    eprintln!("tracking {}", fresh[0]);
                    hash = Some(fresh[0].clone());
                }
                _ => bail!("more than one new torrent appeared; add it by magnet link instead"),
            }
        }

        let hash = hash.as_deref().expect("hash set above");
        match client.get_torrent(&gate, hash).await? {
            None => eprintln!("waiting for metadata…"),
            Some(torrent) => {
                if torrent.is_in_trouble() {
                    bail!("torrent {} entered state {}", torrent.name, torrent.state);
                }
                if torrent.is_complete() {
                    println!();
                    println!("done : {}", torrent.name);
                    println!("path : {}", torrent.content_path);
                    log.info(None, None, format!("completed {}", torrent.name))?;
                    return Ok(());
                }
                print!(
                    "\r{:5.1}%  {:<14}  {:>9}/s  eta {:<9}",
                    torrent.progress * 100.0,
                    torrent.state,
                    human_bytes(torrent.dlspeed.max(0) as u64),
                    fmt_eta(torrent.eta),
                );
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        }
    }
}

pub async fn torrents_cmd(data_dir: &Path, filter: Option<&str>, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let gate = Gate::new(db)?;
    let client = qbit::Qbit::from_settings(&settings)?;
    let torrents = client.list(&gate, filter).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&torrents)?);
        return Ok(());
    }
    if torrents.is_empty() {
        println!("qBittorrent has no torrents");
        return Ok(());
    }
    println!(
        "{:>6}  {:<14}  {:>6}  {:>9}  {:>9}  {:>8}  {}",
        "PROG", "STATE", "SEEDS", "DOWN", "UP", "ETA", "NAME"
    );
    for torrent in &torrents {
        println!(
            "{:>5.1}%  {:<14}  {:>6}  {:>9}  {:>9}  {:>8}  {}",
            torrent.progress * 100.0,
            torrent.state,
            torrent.num_seeds,
            human_bytes(torrent.dlspeed.max(0) as u64),
            human_bytes(torrent.upspeed.max(0) as u64),
            fmt_eta(torrent.eta),
            torrent.name,
        );
    }
    Ok(())
}

/// The info hash behind a bare 40-hex string or a magnet link.
pub fn info_hash_of(target: &str) -> Result<String> {
    if let Some(hash) = qbit::magnet_info_hash(target) {
        return Ok(hash);
    }
    let trimmed = target.trim();
    if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(trimmed.to_lowercase());
    }
    bail!("expected a 40-character info hash or a magnet link, got {target:?}");
}

pub async fn rm_cmd(data_dir: &Path, target: &str, delete_files: bool) -> Result<()> {
    let hash = info_hash_of(target)?;
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let log = EventLog::new(db.clone());
    let gate = Gate::new(db)?;

    let client = qbit::Qbit::from_settings(&settings)?;
    let Some(torrent) = client.get_torrent(&gate, &hash).await? else {
        bail!("qBittorrent does not have a torrent with hash {hash}");
    };
    client.remove(&gate, &hash, delete_files).await?;
    let suffix = if delete_files { " (with files)" } else { "" };
    log.info(None, None, format!("removed {}{suffix}", torrent.name))?;
    println!("removed {}{suffix}", torrent.name);
    Ok(())
}

pub async fn qbit_cmd(data_dir: &Path, command: QbitCommand) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    match command {
        QbitCommand::Test => {
            let gate = Gate::new(db)?;
            let client = qbit::Qbit::from_settings(&settings)?;
            let health = match client.health(&gate).await {
                Ok(health) => health,
                Err(err) if err.is_connection_failure() => {
                    bail!("{}", qbit::setup_help(&settings));
                }
                Err(err) => return Err(err.into()),
            };
            let speeds = client.speeds(&gate).await?;
            println!("url       : {}", settings.qbit_url);
            println!("login     : ok (as {})", settings.qbit_username);
            println!(
                "version   : {} (API {:?})",
                health.version, health.gen
            );
            println!("breaker   : {:?}", gate.breaker_state(Service::Qbit));
            println!(
                "speeds    : {} down, {} up",
                human_bytes(speeds.download.max(0) as u64),
                human_bytes(speeds.upload.max(0) as u64),
            );
            Ok(())
        }
        QbitCommand::Start => {
            let gate = Gate::new(db)?;
            let health = qbit::ensure_running(&gate, &settings).await?;
            println!(
                "qBittorrent {} ({:?}) is listening at {}",
                health.version, health.gen, settings.qbit_url
            );
            Ok(())
        }
        QbitCommand::Password { delete } => {
            if delete {
                secrets::delete(secrets::Secret::QbitPassword)?;
                println!("stored WebUI password removed from the keyring");
                return Ok(());
            }
            eprint!("qBittorrent WebUI password: ");
            use std::io::{IsTerminal, Write};
            std::io::stderr().flush()?;
            let password = if std::io::stdin().is_terminal() {
                rpassword::read_password()?
            } else {
                let mut line = String::new();
                std::io::stdin().read_line(&mut line)?;
                line.trim_end_matches(['\r', '\n']).to_string()
            };
            secrets::set(secrets::Secret::QbitPassword, &password)?;
            println!(
                "stored in the system keyring ({} / {})",
                secrets::Secret::QbitPassword.key(),
                secrets::Secret::QbitPassword.env_var()
            );
            let gate = Gate::new(db)?;
            match qbit::Qbit::from_settings(&settings)?.health(&gate).await {
                Ok(health) => println!("verified  : logged in, qBittorrent {}", health.version),
                Err(err) => eprintln!("warning   : could not verify yet ({err})"),
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_info_hashes_from_both_spellings() {
        let hash = "7acf8fb590b2060dd9c3146ef770169d593433b0";
        assert_eq!(info_hash_of(hash).unwrap(), hash);
        assert_eq!(
            info_hash_of(&format!("magnet:?xt=urn:btih:{hash}&dn=Show")).unwrap(),
            hash,
            "magnets work too"
        );
        assert_eq!(
            info_hash_of(&hash.to_uppercase()).unwrap(),
            hash,
            "case is normalised"
        );
        assert!(info_hash_of("https://nyaa.si/download/1.torrent").is_err());
        assert!(info_hash_of("not-a-hash").is_err());
    }
}
