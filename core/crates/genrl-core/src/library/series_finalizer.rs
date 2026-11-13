use std::fs;
use std::path::Path;

use crate::db::Db;
use crate::error::Result;
use crate::events::{EventLog, Level};
use crate::worker::{FinalizeContext, FinalizeOutcome, FinalizeSuccess};

use super::layout::series_path;
use super::media_finalizer::{collect_files_recursive, is_same_size, transfer_single_file};
use super::select::select_series_pack_files;

pub async fn finalize_series(
    db: &Db,
    event_log: &EventLog,
    context: &FinalizeContext<'_>,
    library_root: &Path,
    title: &str,
    year: Option<i64>,
    source_path: &Path,
) -> Result<FinalizeOutcome> {
    if !source_path.is_dir() {
        let filename = source_path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();
        let parsed = crate::matching::release_parse::parse_media_release(filename);
        let season = parsed.season.unwrap_or(1);
        let episode = parsed.episode.unwrap_or(1.0);
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mkv");
        let target_path = series_path(
            library_root,
            title,
            year,
            season as i64,
            episode as i64,
            ext,
        )?;
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !is_same_size(&target_path, source_path) {
            transfer_single_file(
                db,
                event_log,
                context.job.id,
                context.mal_id,
                source_path,
                &target_path,
            )?;
        }
        mark_media_episode_done(
            db,
            context.job.media_id,
            season,
            episode as i64,
            &target_path.display().to_string(),
        );
        return Ok(FinalizeOutcome::Done(FinalizeSuccess {
            destination_path: target_path.display().to_string(),
            target_episode_covered: true,
        }));
    }

    let mut files = Vec::new();
    collect_files_recursive(source_path, source_path, &mut files)?;
    let pack = select_series_pack_files(&files)?;

    for unmapped in &pack.unmapped_files {
        let _ = event_log.record(
            Level::Warn,
            Some(context.job.id),
            None,
            format!("unmapped file in series pack: {unmapped}"),
        );
    }

    // Read wanted seasons for this series to skip unwanted seasons in multi-season packs
    let wanted_seasons: Vec<u32> = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT DISTINCT season FROM media_episodes WHERE media_id = ?1 AND state = 'wanted'",
        )?;
        let rows = stmt.query_map([context.job.media_id.unwrap_or(0)], |r| r.get(0))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    })?;

    let mut first_dest = None;
    for ep in pack.episodes {
        if !wanted_seasons.is_empty() && !wanted_seasons.contains(&ep.season) {
            let _ = event_log.record(
                Level::Info,
                Some(context.job.id),
                None,
                format!(
                    "skipping file for unwanted season {}: {}",
                    ep.season, ep.file.path
                ),
            );
            continue;
        }

        let ep_src = source_path.join(&ep.file.path);
        let ext = ep_src.extension().and_then(|e| e.to_str()).unwrap_or("mkv");
        let target = series_path(
            library_root,
            title,
            year,
            ep.season as i64,
            ep.episode as i64,
            ext,
        )?;
        if let Some(p) = target.parent() {
            fs::create_dir_all(p)?;
        }
        if !is_same_size(&target, &ep_src) {
            transfer_single_file(
                db,
                event_log,
                context.job.id,
                context.mal_id,
                &ep_src,
                &target,
            )?;
        }
        mark_media_episode_done(
            db,
            context.job.media_id,
            ep.season,
            ep.episode as i64,
            &target.display().to_string(),
        );
        if first_dest.is_none() {
            first_dest = Some(target.display().to_string());
        }
    }

    Ok(FinalizeOutcome::Done(FinalizeSuccess {
        destination_path: first_dest.unwrap_or_default(),
        target_episode_covered: true,
    }))
}

pub fn mark_media_episode_done(
    db: &Db,
    media_id: Option<i64>,
    season: u32,
    episode: i64,
    path: &str,
) {
    if let Some(mid) = media_id {
        let _ = db.with(|c| {
            c.execute(
                "UPDATE media_episodes SET state = 'done', file_path = ?1 WHERE media_id = ?2 AND season = ?3 AND episode = ?4",
                rusqlite::params![path, mid, season, episode],
            )?;
            Ok(())
        });
    }
}
