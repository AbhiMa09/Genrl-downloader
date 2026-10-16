use anyhow::{bail, Result};
use clap::Subcommand;
use genrl_core::http::Gate;
use genrl_core::models::NameSource;
use genrl_core::names::{NameSet, PRIORITY_USER};
use genrl_core::rules::Rules;
use genrl_core::Settings;
use std::path::Path;

use super::common::open_db;

#[derive(Subcommand, Debug)]
pub enum AliasCommand {
    /// Add an alias for a show.
    Add { mal_id: i64, name: String },
    /// Remove an alias for a show.
    Remove { mal_id: i64, name: String },
    /// List all aliases for a show.
    List {
        mal_id: i64,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum LibraryCommand {
    /// Rescan the library folder for existing shows and episodes.
    Rescan,
}

pub fn shows_cmd(data_dir: &Path, status_filter: Option<&str>, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let shows = db.with(|c| {
        let mut sql = String::from(
            "SELECT s.mal_id, s.title, s.english_title, s.start_year, s.total_episodes,
                    s.status, s.synopsis, s.poster_path, s.folder_path, s.auto,
                    COALESCE(m.list_status, 'plan_to_watch'),
                    COALESCE(m.watched_count, 0),
                    COALESCE(m.score, s.score, 0),
                    (SELECT COUNT(*) FROM episodes e WHERE e.mal_id = s.mal_id AND e.state = 'done') AS downloaded_episodes,
                    s.metadata_status, s.metadata_error
             FROM shows s
             LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
             WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(st) = status_filter {
            sql.push_str(" AND (m.list_status = ?1 OR (?2 = 'all'))");
            params.push(Box::new(st.to_string()));
            params.push(Box::new(st.to_string()));
        }
        sql.push_str(" ORDER BY s.title ASC");

        let mut stmt = c.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), |r| {
            let total_eps: Option<i64> = r.get(4)?;
            let airing_st: Option<String> = r.get(5)?;
            let synopsis_txt: Option<String> = r.get(6)?;
            let poster_val: Option<String> = r.get(7)?;
            Ok(serde_json::json!({
                "mal_id": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "english_title": r.get::<_, Option<String>>(2)?,
                "start_year": r.get::<_, Option<i64>>(3)?,
                "total_episodes": total_eps.unwrap_or(0),
                "airing_status": airing_st.unwrap_or_else(|| "Finished".into()),
                "synopsis": synopsis_txt.unwrap_or_default(),
                "poster_url": poster_val,
                "folder_path": r.get::<_, Option<String>>(8)?,
                "auto": r.get::<_, i64>(9)? != 0,
                "status": r.get::<_, String>(10)?,
                "watched_episodes": r.get::<_, i64>(11)?,
                "score": r.get::<_, f64>(12)?,
                "downloaded_episodes": r.get::<_, i64>(13)?,
                "metadata_status": r.get::<_, Option<String>>(14)?.unwrap_or_else(|| "ready".into()),
                "metadata_error": r.get::<_, Option<String>>(15)?,
            }))
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    })?;

    if json {
        println!("{}", serde_json::to_string_pretty(&shows)?);
    } else {
        println!("Library Shows ({} found):", shows.len());
        for s in shows {
            println!("  #{} {} ({})", s["mal_id"], s["title"], s["status"]);
        }
    }
    Ok(())
}

pub async fn show_detail_cmd(data_dir: &Path, mal_id: i64, json: bool) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    let clock = genrl_core::clock::SystemClock;

    // Check if synopsis or poster is missing or pending; trigger on-demand fetch if needed
    let (current_synopsis, current_poster, meta_status): (
        Option<String>,
        Option<String>,
        Option<String>,
    ) = db.with(|c| {
        Ok(c.query_row(
            "SELECT synopsis, poster_path, metadata_status FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((None, None, None)))
    })?;

    let poster_is_missing = current_poster
        .as_deref()
        .map(|p| p.trim().is_empty() || p.starts_with("http") || !Path::new(p).is_file())
        .unwrap_or(true);

    let needs_fetch = current_synopsis
        .as_deref()
        .map(|s| s.trim().is_empty() || s.starts_with('['))
        .unwrap_or(true)
        || poster_is_missing
        || meta_status.as_deref() == Some("pending");

    if needs_fetch && meta_status.as_deref() != Some("failed") {
        if let Ok(gate) = Gate::new(db.clone()) {
            let _ = genrl_core::fetch_show_metadata(
                &db, &gate, &settings, &clock, data_dir, mal_id, false,
            )
            .await;
        }
    }

    let _ = genrl_core::rewrite_show_folder_metadata(&db, mal_id);

    let mut show: serde_json::Value = db.with(|c| {
        let val = c.query_row(
            "SELECT s.mal_id, s.title, s.english_title, s.start_year, s.total_episodes,
                    s.status, s.synopsis, s.poster_path, s.folder_path, s.auto,
                    COALESCE(m.list_status, 'plan_to_watch'),
                    COALESCE(m.watched_count, 0),
                    COALESCE(m.score, s.score, 0),
                    (SELECT COUNT(*) FROM episodes e WHERE e.mal_id = s.mal_id AND e.state = 'done'),
                    COALESCE(s.type, 'tv'),
                    COALESCE(s.season, 1),
                    COALESCE(s.episode_offset, 0),
                    s.metadata_status,
                    s.metadata_updated_at,
                    s.metadata_next_retry_at,
                    s.metadata_error
             FROM shows s
             LEFT JOIN mal_entries m ON m.mal_id = s.mal_id
             WHERE s.mal_id = ?1",
            [mal_id],
            |r| {
                let total_eps: Option<i64> = r.get(4)?;
                let airing_st: Option<String> = r.get(5)?;
                let synopsis_txt: Option<String> = r.get(6)?;
                let poster_val: Option<String> = r.get(7)?;
                Ok(serde_json::json!({
                    "mal_id": r.get::<_, i64>(0)?,
                    "title": r.get::<_, String>(1)?,
                    "english_title": r.get::<_, Option<String>>(2)?,
                    "start_year": r.get::<_, Option<i64>>(3)?,
                    "total_episodes": total_eps.unwrap_or(0),
                    "airing_status": airing_st.unwrap_or_else(|| "Finished".into()),
                    "synopsis": synopsis_txt.unwrap_or_default(),
                    "poster_url": poster_val,
                    "folder_path": r.get::<_, Option<String>>(8)?,
                    "auto": r.get::<_, i64>(9)? != 0,
                    "status": r.get::<_, String>(10)?,
                    "watched_episodes": r.get::<_, i64>(11)?,
                    "score": r.get::<_, f64>(12)?,
                    "downloaded_episodes": r.get::<_, i64>(13)?,
                    "media_type": r.get::<_, String>(14)?,
                    "season": r.get::<_, i64>(15)?,
                    "episode_offset": r.get::<_, i64>(16)?,
                    "metadata_status": r.get::<_, Option<String>>(17)?.unwrap_or_else(|| "ready".into()),
                    "metadata_updated_at": r.get::<_, Option<i64>>(18)?,
                    "metadata_next_retry_at": r.get::<_, Option<i64>>(19)?,
                    "metadata_error": r.get::<_, Option<String>>(20)?,
                }))
            },
        )?;
        Ok(val)
    })?;

    // Check synopsis. If empty, check folder_path/synopsis.txt (ignoring placeholder lines starting with [)
    let current_synopsis = show["synopsis"].as_str().unwrap_or("").trim().to_string();
    if current_synopsis.is_empty() || current_synopsis.starts_with('[') {
        if let Some(folder) = show["folder_path"].as_str() {
            let syn_file = Path::new(folder).join("synopsis.txt");
            if syn_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&syn_file) {
                    let trimmed = content.trim().to_string();
                    if !trimmed.is_empty() && !trimmed.starts_with('[') {
                        let _ = db.with(|c| {
                            c.execute(
                                "UPDATE shows SET synopsis = ?1 WHERE mal_id = ?2",
                                rusqlite::params![trimmed, mal_id],
                            )?;
                            Ok(())
                        });
                        if let Some(obj) = show.as_object_mut() {
                            obj.insert("synopsis".to_string(), serde_json::Value::String(trimmed));
                        }
                    }
                }
            }
        }
    }

    // Load aliases and Japanese title from show_names
    let (aliases, japanese_title) = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT name, source, priority FROM show_names WHERE mal_id = ?1 ORDER BY priority, id"
        )?;
        let mut names: Vec<String> = Vec::new();
        let mut ja: Option<String> = None;
        let rows = stmt.query_map([mal_id], |r| {
            let name: String = r.get(0)?;
            let source: String = r.get(1)?;
            let priority: i64 = r.get(2)?;
            Ok((name, source, priority))
        })?;
        for row in rows {
            let (name, _, _) = row?;
            let has_cjk = name.chars().any(|ch| {
                matches!(ch, '\u{3040}'..='\u{309f}' | '\u{30a0}'..='\u{30ff}' | '\u{4e00}'..='\u{9faf}')
            });
            if has_cjk && ja.is_none() {
                ja = Some(name.clone());
            }
            if !names.contains(&name) {
                names.push(name);
            }
        }
        Ok((names, ja))
    })?;

    // Load per-show rules
    let rules = Rules::for_show(&db, mal_id).unwrap_or_default();
    let rules_val = serde_json::to_value(&rules).unwrap_or(serde_json::Value::Null);

    // Query episodes
    let is_movie = show["media_type"].as_str() == Some("movie");
    let show_title = show["title"].as_str().unwrap_or("Show");

    let episodes = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT e.id, e.number, e.state, e.file_path, e.size, t.name, t.\"group\", t.resolution
             FROM episodes e
             LEFT JOIN torrents t ON t.id = e.torrent_id
             WHERE e.mal_id = ?1
             ORDER BY e.number ASC",
        )?;
        let rows = stmt.query_map([mal_id], |r| {
            let id = r.get::<_, i64>(0)?;
            let num = r.get::<_, f64>(1)?;
            let state = r.get::<_, String>(2)?;
            let path = r.get::<_, Option<String>>(3)?;
            let size = r.get::<_, Option<i64>>(4)?;
            let torrent_name = r.get::<_, Option<String>>(5)?;
            let group = r.get::<_, Option<String>>(6)?;
            let res = r.get::<_, Option<String>>(7)?;

            let num_int = num as i64;
            let title = if is_movie {
                format!("Feature Film: {show_title}")
            } else {
                format!("Episode {num_int}")
            };

            let release_name = torrent_name
                .clone()
                .or_else(|| {
                    path.as_ref().and_then(|p| {
                        Path::new(p)
                            .file_name()
                            .and_then(|f| f.to_str())
                            .map(|s| s.to_string())
                    })
                })
                .unwrap_or_else(|| format!("[Genrl] {show_title} - {:02}", num_int));

            let ext = path
                .as_ref()
                .and_then(|p| Path::new(p).extension().and_then(|e| e.to_str()))
                .unwrap_or("mkv");
            let container = match ext.to_lowercase().as_str() {
                "mp4" => "MPEG-4 (MP4)",
                "webm" => "WebM",
                _ => "Matroska (MKV)",
            };

            let resolution = res.unwrap_or_else(|| "1080p".to_string());
            let video_track = format!("{resolution} HEVC");
            let audio_track = "Japanese (Stereo / 5.1)".to_string();
            let subtitles = "ENG, JPN".to_string();

            let crc32 = release_name
                .rfind('[')
                .and_then(|start| {
                    release_name[start..].find(']').and_then(|end| {
                        let candidate = &release_name[start + 1..start + end];
                        if candidate.len() == 8 && candidate.chars().all(|c| c.is_ascii_hexdigit())
                        {
                            Some(candidate.to_uppercase())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| "-".to_string());

            Ok(serde_json::json!({
                "id": id,
                "number": num_int,
                "title": title,
                "state": state,
                "is_downloaded": state == "done",
                "file_path": path,
                "size_bytes": size,
                "release_name": release_name,
                "group": group.unwrap_or_else(|| "Auto".to_string()),
                "container": container,
                "video_track": video_track,
                "primary_audio": audio_track,
                "subtitles": subtitles,
                "crc32": crc32,
                "chapters": 0,
                "duration": if is_movie { "-" } else { "24m" },
            }))
        })?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    })?;

    let on_disk_bytes: i64 = episodes
        .iter()
        .filter_map(|e| e["size_bytes"].as_i64())
        .sum();

    if let Some(obj) = show.as_object_mut() {
        obj.insert(
            "aliases".to_string(),
            serde_json::Value::Array(aliases.into_iter().map(serde_json::Value::String).collect()),
        );
        if let Some(ja) = japanese_title {
            obj.insert("japanese_title".to_string(), serde_json::Value::String(ja));
        }
        obj.insert("rules".to_string(), rules_val);
        obj.insert(
            "on_disk_bytes".to_string(),
            serde_json::Value::Number(serde_json::Number::from(on_disk_bytes)),
        );
        obj.insert("episodes".to_string(), serde_json::Value::Array(episodes));
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&show)?);
    } else {
        println!("Show #{mal_id}: {}", show["title"]);
    }
    Ok(())
}

pub fn alias_cmd(data_dir: &Path, command: AliasCommand) -> Result<()> {
    let db = open_db(data_dir)?;
    match command {
        AliasCommand::Add { mal_id, name } => {
            let added = NameSet::add(&db, mal_id, &name, NameSource::User, PRIORITY_USER)?;
            if added {
                println!("Added alias '{name}' to show #{mal_id}");
            } else {
                println!("Alias '{name}' already exists for show #{mal_id}");
            }
        }
        AliasCommand::Remove { mal_id, name } => {
            let removed = NameSet::remove(&db, mal_id, &name)?;
            if removed {
                println!("Removed alias '{name}' from show #{mal_id}");
            } else {
                println!("Alias '{name}' not found for show #{mal_id}");
            }
        }
        AliasCommand::List { mal_id, json } => {
            let set = NameSet::load(&db, mal_id)?;
            if json {
                let list: Vec<String> = set.entries.into_iter().map(|e| e.name).collect();
                println!("{}", serde_json::to_string_pretty(&list)?);
            } else {
                println!("Aliases for show #{mal_id}:");
                for entry in set.entries {
                    println!("  {} ({})", entry.name, entry.source.as_str());
                }
            }
        }
    }
    Ok(())
}

pub fn offset_cmd(data_dir: &Path, mal_id: i64, offset: i64) -> Result<()> {
    let db = open_db(data_dir)?;
    db.with(|c| {
        c.execute(
            "UPDATE shows SET episode_offset = ?1 WHERE mal_id = ?2",
            rusqlite::params![offset, mal_id],
        )?;
        Ok(())
    })?;
    println!("Set episode offset for show #{mal_id} to {offset}");
    Ok(())
}

pub fn library_rescan_cmd(data_dir: &Path) -> Result<()> {
    let db = open_db(data_dir)?;
    let settings = Settings::load(&db)?;
    if settings.library_folder.is_empty() {
        bail!("no library folder configured; set library_folder in settings first");
    }
    let p = Path::new(&settings.library_folder);
    if !p.exists() {
        bail!(
            "configured library folder '{}' does not exist",
            settings.library_folder
        );
    }
    let report = genrl_core::library::rescan_library(&db, p)?;
    println!(
        "rescanned '{}': {} show(s) adopted, {} episode(s) indexed",
        p.display(),
        report.shows_adopted,
        report.episodes_found
    );
    Ok(())
}
