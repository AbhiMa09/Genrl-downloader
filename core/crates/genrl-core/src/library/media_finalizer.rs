use std::fs;
use std::path::Path;

use rusqlite::OptionalExtension;

use crate::db::Db;
use crate::error::{Error, Result};
use crate::events::{EventLog, Level};
use crate::questions::{Answer, Choice, ExpectedShow, Payload, QuestionStore};
use crate::settings::Settings;
use crate::worker::{FinalizeContext, FinalizeOutcome, FinalizeSuccess};

use super::fscheck::same_volume;
use super::layout::movie_path;
use super::select::{select_movie_files, MovieSelectionOutcome, TorrentFile};

/// Finalize a non-anime media download (movie or tv series).
pub async fn finalize_media(
    db: &Db,
    context: &FinalizeContext<'_>,
    settings: &Settings,
) -> Result<FinalizeOutcome> {
    let event_log = EventLog::new(db.clone());
    let source_path = Path::new(context.content_path);
    if !source_path.exists() {
        return Err(Error::Other(format!(
            "finished content path {} does not exist on disk",
            context.content_path
        )));
    }

    let library_root = if settings.library_folder.trim().is_empty() {
        crate::locations::default_library_dir()
    } else {
        crate::locations::resolve_home_dir(&settings.library_folder)
    };
    if !library_root.exists() {
        let _ = fs::create_dir_all(&library_root);
    }

    let media_id = context.job.media_id;
    let (title, year, kind) = match media_id {
        Some(mid) => {
            let row: Option<(String, Option<i64>, String)> = db.with(|c| {
                Ok(c.query_row(
                    "SELECT title, year, kind FROM media WHERE id = ?1",
                    [mid],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<i64>>(1)?,
                            r.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?)
            })?;
            row.unwrap_or_else(|| {
                let stem = source_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled");
                (stem.to_string(), None, "movie".to_string())
            })
        }
        None => {
            let stem = source_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled");
            (
                stem.to_string(),
                None,
                context
                    .job
                    .media_kind
                    .clone()
                    .unwrap_or_else(|| "movie".into()),
            )
        }
    };

    if kind == "movie" || context.job.media_kind.as_deref() == Some("movie") {
        finalize_movie(
            db,
            &event_log,
            context,
            &library_root,
            &title,
            year,
            source_path,
        )
        .await
    } else {
        super::series_finalizer::finalize_series(
            db,
            &event_log,
            context,
            &library_root,
            &title,
            year,
            source_path,
        )
        .await
    }
}

async fn finalize_movie(
    db: &Db,
    event_log: &EventLog,
    context: &FinalizeContext<'_>,
    library_root: &Path,
    title: &str,
    year: Option<i64>,
    source_path: &Path,
) -> Result<FinalizeOutcome> {
    // 1. Select main video and sidecar files
    let (main_video_path, sidecars) = if source_path.is_dir() {
        let mut files = Vec::new();
        collect_files_recursive(source_path, source_path, &mut files)?;
        match select_movie_files(&files)? {
            MovieSelectionOutcome::AmbiguousNearEqualVideos { video_a, video_b } => {
                let payload = Payload::MatchReview {
                    expected: ExpectedShow {
                        mal_id: 0,
                        title: title.to_string(),
                        english_title: None,
                        year,
                    },
                    options: vec![],
                    reason: format!(
                        "two near-equal videos found in movie: {} vs {}",
                        video_a.path, video_b.path
                    ),
                };
                let mal_id_opt = if context.mal_id > 0 {
                    Some(context.mal_id)
                } else {
                    None
                };
                let qid = QuestionStore::ask(
                    db,
                    Some(context.job.id),
                    mal_id_opt,
                    &payload,
                    crate::db::now_unix(),
                    Some(3600),
                )?;
                return Ok(FinalizeOutcome::NeedsInput { question_id: qid });
            }
            MovieSelectionOutcome::Selected(sel) => {
                let main_p = source_path.join(&sel.main_video.path);
                let subs = sel
                    .subtitles
                    .into_iter()
                    .map(|s| (source_path.join(&s.source_path), s.lang_suffix))
                    .collect();
                (main_p, subs)
            }
        }
    } else {
        (source_path.to_path_buf(), vec![])
    };

    let ext = main_video_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mkv");
    let target_path = movie_path(library_root, title, year, ext)?;

    // 2. Security / Traversal check (canonicalization)
    let parent_dir = target_path
        .parent()
        .ok_or_else(|| Error::Other("invalid movie target path".into()))?;
    fs::create_dir_all(parent_dir)?;
    if let (Ok(can_root), Ok(can_parent)) = (library_root.canonicalize(), parent_dir.canonicalize())
    {
        if !can_parent.starts_with(&can_root) {
            return Err(Error::Other(format!(
                "path traversal detected: target '{}' escapes library root '{}'",
                target_path.display(),
                library_root.display()
            )));
        }
    }

    // 3. Collision handling
    if target_path.exists() {
        let target_size = fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0);
        let source_size = fs::metadata(&main_video_path).map(|m| m.len()).unwrap_or(0);

        if target_size == source_size && target_size > 0 {
            return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                destination_path: target_path.display().to_string(),
                target_episode_covered: true,
            }));
        }

        // Different size: check if question already exists/answered
        let question: Option<(i64, Option<String>)> = db.with(|c| {
            c.query_row(
                "SELECT id, answer FROM questions WHERE job_id = ?1 AND kind = 'file_exists' ORDER BY id DESC LIMIT 1",
                [context.job.id],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
            ).optional().map_err(Into::into)
        })?;

        match question {
            Some((qid, None)) => return Ok(FinalizeOutcome::NeedsInput { question_id: qid }),
            Some((_qid, Some(ans_json))) => {
                let choice = serde_json::from_str::<Answer>(&ans_json)
                    .map(|a| a.choice)
                    .unwrap_or(Choice::Keep);
                match choice {
                    Choice::Keep | Choice::Wait => {
                        return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                            destination_path: target_path.display().to_string(),
                            target_episode_covered: true,
                        }));
                    }
                    Choice::Replace => {
                        let _ = fs::remove_file(&target_path);
                    }
                    _ => {
                        return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                            destination_path: target_path.display().to_string(),
                            target_episode_covered: true,
                        }));
                    }
                }
            }
            None => {
                if target_size > 0 {
                    return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                        destination_path: target_path.display().to_string(),
                        target_episode_covered: true,
                    }));
                }
            }
        }
    }

    // 4. Transfer main video
    transfer_single_file(
        db,
        event_log,
        context.job.id,
        context.mal_id,
        &main_video_path,
        &target_path,
    )?;

    // 5. Transfer sidecar subtitles
    let target_stem = target_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Movie");
    for (sub_src, lang_suffix) in sidecars {
        if !sub_src.exists() {
            continue;
        }
        let sub_ext = sub_src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("srt");
        let dest_sub_name = match lang_suffix {
            Some(lang) => format!("{target_stem}.{lang}.{sub_ext}"),
            None => format!("{target_stem}.{sub_ext}"),
        };
        let dest_sub_path = parent_dir.join(dest_sub_name);
        let _ = transfer_single_file(
            db,
            event_log,
            context.job.id,
            context.mal_id,
            &sub_src,
            &dest_sub_path,
        );
    }

    Ok(FinalizeOutcome::Done(FinalizeSuccess {
        destination_path: target_path.display().to_string(),
        target_episode_covered: true,
    }))
}

pub(crate) fn is_same_size(a: &Path, b: &Path) -> bool {
    let sa = fs::metadata(a).map(|m| m.len()).unwrap_or(0);
    let sb = fs::metadata(b).map(|m| m.len()).unwrap_or(0);
    sa > 0 && sa == sb
}

pub fn transfer_single_file(
    _db: &Db,
    event_log: &EventLog,
    job_id: i64,
    mal_id: i64,
    source: &Path,
    target: &Path,
) -> Result<()> {
    let same = same_volume(source, target).unwrap_or(false);
    if same {
        if let Err(e) = fs::hard_link(source, target) {
            tracing::warn!("hard_link failed ({e}), falling back to copy");
            fs::copy(source, target)?;
            let _ = event_log.record(
                Level::Warn,
                Some(job_id),
                Some(mal_id),
                "hardlink not possible across volumes, file was copied",
            );
        }
    } else {
        fs::copy(source, target)?;
        let _ = event_log.record(
            Level::Warn,
            Some(job_id),
            Some(mal_id),
            "hardlink not possible, file was copied",
        );
    }
    crate::platform::set_file_permissions(target, 0o644)?;
    Ok(())
}

pub(crate) fn collect_files_recursive(
    root: &Path,
    dir: &Path,
    out: &mut Vec<TorrentFile>,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            collect_files_recursive(root, &p, out)?;
        } else if let Ok(meta) = entry.metadata() {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .to_string();
            out.push(TorrentFile {
                path: rel,
                size_bytes: meta.len(),
            });
        }
    }
    Ok(())
}
