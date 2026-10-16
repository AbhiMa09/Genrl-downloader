use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use rusqlite::OptionalExtension;

use crate::error::{Error, Result};
use crate::events::EventLog;
use crate::models::ShowType;
use crate::questions::{Answer, Choice, Payload, QuestionStore};
use crate::settings::Settings;
use crate::worker::{FinalizeContext, FinalizeOutcome, FinalizeSuccess, Finalizer};

use super::folders::{resolve_show_folder, transfer_file};
use super::formatting::{
    format_episode_filename, format_nfo, format_synopsis, is_placeholder_synopsis,
};
use super::icons::{apply_folder_icon, create_folder_png, write_solid_png};
use super::service::LibraryFinalizer;
use super::types::{FOLDER_ICON_FILE, SYNOPSIS_FILE, VIDEO_EXTENSIONS};

impl Finalizer for LibraryFinalizer {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>> {
        Box::pin(async move {
            let settings = Settings::load(&self.db)?;
            let source_path = Path::new(context.content_path);
            if !source_path.exists() {
                return Err(Error::Other(format!(
                    "finished content path {} does not exist on disk",
                    context.content_path
                )));
            }

            if source_path.is_dir() {
                return self.finalize_batch(context, source_path, &settings).await;
            }

            // Verify extension allowlist
            let ext = source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            if !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                return Err(Error::Other(format!(
                    "file extension .{} is not an allowed video format",
                    ext
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

            // Load show information from database
            let (title, english_title, show_type_str, start_year, season, existing_synopsis, existing_poster, existing_score, existing_episodes, existing_status) = self.db.with(|c| {
                Ok(c.query_row(
                    "SELECT title, english_title, type, start_year, season, synopsis, poster_path, score, total_episodes, status FROM shows WHERE mal_id = ?1",
                    [context.mal_id],
                    |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, Option<String>>(1)?,
                            r.get::<_, String>(2)?,
                            r.get::<_, Option<i64>>(3)?,
                            r.get::<_, i64>(4)?,
                            r.get::<_, Option<String>>(5)?,
                            r.get::<_, Option<String>>(6)?,
                            r.get::<_, Option<f64>>(7)?,
                            r.get::<_, Option<i64>>(8)?,
                            r.get::<_, Option<String>>(9)?,
                        ))
                    },
                )?)
            })?;
            let show_type = ShowType::parse(&show_type_str);

            // 1. Resolve Show Folder
            let show_dir =
                resolve_show_folder(&self.db, &library_root, context.mal_id, &title, start_year)?;

            // 2. Write synopsis.txt, NFO, and folder.png if not present
            let synopsis_path = show_dir.join(SYNOPSIS_FILE);
            let folder_png_path = show_dir.join(FOLDER_ICON_FILE);
            let nfo_path = if show_type == ShowType::Movie {
                show_dir.join("movie.nfo")
            } else {
                show_dir.join("tvshow.nfo")
            };

            if !synopsis_path.exists() || !folder_png_path.exists() || !nfo_path.exists() {
                let meta = if existing_synopsis.is_none() || existing_poster.is_none() {
                    self.fetch_metadata(context.mal_id, &settings).await
                } else {
                    None
                };
                let now_date = {
                    let ts = self.clock.now();
                    format!("Unix timestamp {ts}")
                };

                let syn_text = match (&meta, &existing_synopsis) {
                    (Some(m), _) => {
                        let genres: Vec<String> = m.genres.iter().map(|g| g.name.clone()).collect();
                        format_synopsis(
                            m.title.as_deref().unwrap_or(&title),
                            m.title_english.as_deref().or(english_title.as_deref()),
                            m.title_japanese.as_deref(),
                            m.show_type.as_deref().unwrap_or(&show_type_str),
                            m.episodes.or(existing_episodes),
                            m.status.as_deref().or(existing_status.as_deref()),
                            m.aired.as_ref().and_then(|a| a.string.as_deref()),
                            m.score.or(existing_score),
                            &genres,
                            m.synopsis.as_deref().or(existing_synopsis.as_deref()),
                            context.mal_id,
                            &now_date,
                        )
                    }
                    (None, Some(syn)) => format_synopsis(
                        &title,
                        english_title.as_deref(),
                        None,
                        &show_type_str,
                        existing_episodes,
                        existing_status.as_deref(),
                        None,
                        existing_score,
                        &[],
                        Some(syn.as_str()),
                        context.mal_id,
                        &now_date,
                    ),
                    (None, None) => format_synopsis(
                        &title,
                        english_title.as_deref(),
                        None,
                        &show_type_str,
                        existing_episodes,
                        existing_status.as_deref(),
                        None,
                        existing_score,
                        &[],
                        None,
                        context.mal_id,
                        &now_date,
                    ),
                };

                let should_write_syn = !synopsis_path.exists() || {
                    if let Ok(c) = fs::read_to_string(&synopsis_path) {
                        is_placeholder_synopsis(&c) && !is_placeholder_synopsis(&syn_text)
                    } else {
                        false
                    }
                };
                if should_write_syn {
                    let _ = fs::write(&synopsis_path, &syn_text);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ =
                            fs::set_permissions(&synopsis_path, fs::Permissions::from_mode(0o644));
                    }
                }

                // NFO metadata file
                let plot = meta
                    .as_ref()
                    .and_then(|m| m.synopsis.as_deref())
                    .or(existing_synopsis.as_deref())
                    .unwrap_or("");
                let should_write_nfo = !nfo_path.exists() || {
                    if let Ok(c) = fs::read_to_string(&nfo_path) {
                        (c.contains("<plot></plot>") || c.contains("<plot/>"))
                            && !plot.trim().is_empty()
                    } else {
                        false
                    }
                };
                if should_write_nfo {
                    let tag = if show_type == ShowType::Movie {
                        "movie"
                    } else {
                        "tvshow"
                    };
                    let score_val = meta.as_ref().and_then(|m| m.score).or(existing_score);
                    let nfo_content = format_nfo(
                        tag,
                        &title,
                        english_title.as_deref(),
                        plot,
                        score_val,
                        start_year,
                        context.mal_id,
                    );
                    let _ = fs::write(&nfo_path, nfo_content);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let _ = fs::set_permissions(&nfo_path, fs::Permissions::from_mode(0o644));
                    }
                }

                // Poster & folder icon
                if !folder_png_path.exists() {
                    let poster_url = meta
                        .as_ref()
                        .and_then(|m| {
                            m.images.as_ref().and_then(|imgs| {
                                imgs.jpg
                                    .as_ref()
                                    .and_then(|j| {
                                        j.large_image_url.clone().or_else(|| j.image_url.clone())
                                    })
                                    .or_else(|| {
                                        imgs.webp.as_ref().and_then(|w| {
                                            w.large_image_url
                                                .clone()
                                                .or_else(|| w.image_url.clone())
                                        })
                                    })
                            })
                        })
                        .or_else(|| {
                            existing_poster.as_ref().and_then(|p| {
                                if p.starts_with("http://") || p.starts_with("https://") {
                                    Some(p.clone())
                                } else {
                                    None
                                }
                            })
                        });

                    let master_poster = match poster_url {
                        Some(url) => self.download_poster(context.mal_id, &url).await,
                        None => {
                            if let Some(ref local_p) = existing_poster {
                                let pb = PathBuf::from(local_p);
                                if pb.exists() {
                                    Some(pb)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    };

                    let poster_to_use = master_poster.unwrap_or_else(|| {
                        let placeholder = self.data_dir.join("placeholder_poster.png");
                        if !placeholder.exists() {
                            let _ = write_solid_png(&placeholder, 64, 64, 30, 20, 50);
                        }
                        placeholder
                    });

                    let _ =
                        create_folder_png(&poster_to_use, &folder_png_path, settings.icon_style);
                    let _ =
                        apply_folder_icon(settings.folder_icon_method, &show_dir, &folder_png_path);
                }
            }

            // 3. Determine Episode Filename
            let ep_number = self.db.with(|c| {
                Ok(c.query_row(
                    "SELECT number FROM episodes WHERE id = ?1",
                    [context.episode_id],
                    |r| r.get::<_, f64>(0),
                )?)
            })?;

            let filename = if show_type == ShowType::Movie {
                format_episode_filename(
                    &settings.movie_naming_pattern,
                    &title,
                    season,
                    ep_number,
                    start_year,
                    &ext,
                )
            } else {
                format_episode_filename(
                    &settings.naming_pattern,
                    &title,
                    season,
                    ep_number,
                    start_year,
                    &ext,
                )
            };

            let target_path = show_dir.join(&filename);

            // Ensure destination sits inside library root (Path traversal check)
            let can_target = target_path
                .canonicalize()
                .unwrap_or_else(|_| target_path.clone());
            if let Ok(can_root) = library_root.canonicalize() {
                if !can_target.starts_with(&can_root) {
                    return Err(Error::Other(format!(
                        "path traversal detected: target '{}' is outside library root '{}'",
                        target_path.display(),
                        library_root.display()
                    )));
                }
            }

            // 4. Collision check
            if target_path.exists() {
                // If identical file (same inode/hardlink or same path), already filed
                if let (Ok(m1), Ok(m2)) = (fs::metadata(source_path), fs::metadata(&target_path)) {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::MetadataExt;
                        if m1.dev() == m2.dev() && m1.ino() == m2.ino() {
                            return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                destination_path: target_path.display().to_string(),
                            }));
                        }
                    }
                }

                // Check questions table for an existing answer
                let question: Option<(i64, Option<String>)> = self.db.with(|c| {
                    Ok(c.query_row(
                        "SELECT id, answer FROM questions WHERE job_id = ?1 AND kind = 'file_exists'",
                        [context.job.id],
                        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
                    )
                    .optional()?)
                })?;

                match question {
                    Some((_qid, Some(ans_json))) => {
                        let answer: Answer = serde_json::from_str(&ans_json).unwrap_or(Answer {
                            choice: Choice::Replace,
                            always: false,
                        });
                        match answer.choice {
                            Choice::Keep => {
                                // Keep existing file on disk as the episode file
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: target_path.display().to_string(),
                                }));
                            }
                            Choice::Replace => {
                                let _ = fs::remove_file(&target_path);
                                transfer_file(source_path, &target_path, settings.move_mode)?;
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: target_path.display().to_string(),
                                }));
                            }
                            Choice::KeepBoth => {
                                let stem = target_path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("episode");
                                let alt_filename = format!("{stem} (v2).{ext}");
                                let alt_path = show_dir.join(alt_filename);
                                transfer_file(source_path, &alt_path, settings.move_mode)?;
                                return Ok(FinalizeOutcome::Done(FinalizeSuccess {
                                    destination_path: alt_path.display().to_string(),
                                }));
                            }
                            _ => {}
                        }
                    }
                    Some((qid, None)) => {
                        // Already asked, still awaiting answer
                        return Ok(FinalizeOutcome::NeedsInput { question_id: qid });
                    }
                    None => {
                        // Ask question in Inbox
                        let payload = Payload::FileExists {
                            episode: ep_number,
                            existing: target_path.display().to_string(),
                            candidate: context.content_path.to_string(),
                        };
                        let now = self.clock.now();
                        let qid = QuestionStore::ask(
                            &self.db,
                            Some(context.job.id),
                            Some(context.mal_id),
                            &payload,
                            now,
                            Some(i64::from(settings.question_expiry_hours) * 3600),
                        )?;
                        EventLog::new(self.db.clone()).warn(
                            Some(context.job.id),
                            Some(context.mal_id),
                            format!("destination file exists: asked question #{qid}"),
                        )?;
                        return Ok(FinalizeOutcome::NeedsInput { question_id: qid });
                    }
                }
            }

            // 5. Transfer file into show folder
            transfer_file(source_path, &target_path, settings.move_mode)?;

            Ok(FinalizeOutcome::Done(FinalizeSuccess {
                destination_path: target_path.display().to_string(),
            }))
        })
    }
}
