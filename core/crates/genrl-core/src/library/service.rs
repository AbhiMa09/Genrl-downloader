use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::http::{Gate, GetOptions, Service};
use crate::models::ShowType;
use crate::settings::Settings;
use crate::worker::{FinalizeContext, FinalizeOutcome, FinalizeSuccess};

use super::folders::{resolve_show_folder, transfer_file};
use super::formatting::{
    format_episode_filename, format_nfo, format_synopsis, is_placeholder_synopsis,
};
use super::icons::{apply_folder_icon, create_folder_png, write_solid_png};
use super::types::{
    JikanAnimeData, JikanResponse, FOLDER_ICON_FILE, SYNOPSIS_FILE, VIDEO_EXTENSIONS,
};

/// The production Finalizer that renames, moves, and decorates completed downloads.
pub struct LibraryFinalizer {
    pub(crate) db: Db,
    pub(crate) gate: Arc<Gate>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) data_dir: PathBuf,
}

impl LibraryFinalizer {
    pub fn new(db: Db, gate: Arc<Gate>, clock: Arc<dyn Clock>, data_dir: PathBuf) -> Self {
        LibraryFinalizer {
            db,
            gate,
            clock,
            data_dir,
        }
    }

    /// Retrieve Jikan metadata with fallback to shows table.
    pub(crate) async fn fetch_metadata(
        &self,
        mal_id: i64,
        settings: &Settings,
    ) -> Option<JikanAnimeData> {
        let url = format!(
            "{}/anime/{}",
            settings.jikan_base_url.trim_end_matches('/'),
            mal_id
        );
        let opts = GetOptions {
            ttl: Some(Duration::from_secs(settings.jikan_cache_secs as u64)),
            ..Default::default()
        };

        match self.gate.get_with(Service::Jikan, &url, &opts).await {
            Ok(fetched) => match fetched.json::<JikanResponse>() {
                Ok(resp) => Some(resp.data),
                Err(err) => {
                    tracing::warn!(mal_id, "failed to parse Jikan metadata: {err}");
                    None
                }
            },
            Err(err) => {
                tracing::warn!(mal_id, "failed to fetch Jikan metadata: {err}");
                None
            }
        }
    }

    /// Download and save the show poster locally.
    pub(crate) async fn download_poster(&self, mal_id: i64, url: &str) -> Option<PathBuf> {
        let poster_dir = self.data_dir.join("posters");
        let _ = fs::create_dir_all(&poster_dir);
        let dst = poster_dir.join(format!("{mal_id}.jpg"));

        if dst.exists() {
            return Some(dst);
        }

        match self
            .gate
            .get_with(Service::Images, url, &GetOptions::default())
            .await
        {
            Ok(fetched) => {
                if fs::write(&dst, &fetched.body).is_ok() {
                    Some(dst)
                } else {
                    None
                }
            }
            Err(err) => {
                tracing::warn!(mal_id, url, "failed to download poster: {err}");
                None
            }
        }
    }

    /// Finalize a batch directory containing multiple episode video files.
    pub(crate) async fn finalize_batch(
        &self,
        context: &FinalizeContext<'_>,
        batch_dir: &Path,
        settings: &Settings,
    ) -> Result<FinalizeOutcome> {
        let library_root = if settings.library_folder.trim().is_empty() {
            crate::locations::default_library_dir()
        } else {
            crate::locations::resolve_home_dir(&settings.library_folder)
        };
        if !library_root.exists() {
            let _ = fs::create_dir_all(&library_root);
        }

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

        let show_dir =
            resolve_show_folder(&self.db, &library_root, context.mal_id, &title, start_year)?;

        // Ensure synopsis, NFO, and folder icon
        let synopsis_path = show_dir.join(SYNOPSIS_FILE);
        let folder_png_path = show_dir.join(FOLDER_ICON_FILE);
        let nfo_path = if show_type == ShowType::Movie {
            show_dir.join("movie.nfo")
        } else {
            show_dir.join("tvshow.nfo")
        };

        if !synopsis_path.exists() || !folder_png_path.exists() || !nfo_path.exists() {
            let meta = if existing_synopsis.is_none() || existing_poster.is_none() {
                self.fetch_metadata(context.mal_id, settings).await
            } else {
                None
            };
            let now_date = format!("Unix timestamp {}", self.clock.now());
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
                    let _ = fs::set_permissions(&synopsis_path, fs::Permissions::from_mode(0o644));
                }
            }

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
                                        w.large_image_url.clone().or_else(|| w.image_url.clone())
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
                let _ = create_folder_png(&poster_to_use, &folder_png_path, settings.icon_style);
                let _ = apply_folder_icon(settings.folder_icon_method, &show_dir, &folder_png_path);
            }
        }

        // Collect all video files recursively in batch_dir
        let mut video_files = Vec::new();
        let mut to_visit = vec![batch_dir.to_path_buf()];
        while let Some(dir) = to_visit.pop() {
            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        to_visit.push(path);
                    } else if path.is_file() {
                        let ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        if VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                            video_files.push(path);
                        }
                    }
                }
            }
        }

        video_files.sort();
        let mut primary_dest = String::new();

        for file_path in video_files {
            let fname = file_path.file_name().and_then(|f| f.to_str()).unwrap_or("");
            let parsed = crate::filename::parse_release(fname);
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("mkv")
                .to_ascii_lowercase();

            let ep_num = parsed.episode.unwrap_or(1.0);
            let target_filename = if show_type == ShowType::Movie {
                format_episode_filename(
                    &settings.movie_naming_pattern,
                    &title,
                    season,
                    ep_num,
                    start_year,
                    &ext,
                )
            } else {
                format_episode_filename(
                    &settings.naming_pattern,
                    &title,
                    season,
                    ep_num,
                    start_year,
                    &ext,
                )
            };

            let target_path = show_dir.join(&target_filename);
            let file_size = fs::metadata(&file_path)
                .map(|m| m.len() as i64)
                .unwrap_or(0);
            if !target_path.exists() {
                let _ = transfer_file(&file_path, &target_path, settings.move_mode);
            }

            let final_dest = target_path.display().to_string();
            if primary_dest.is_empty() {
                primary_dest = final_dest.clone();
            }

            // Update database episode row
            let _ = self.db.with(|c| {
                c.execute(
                    "INSERT INTO episodes (mal_id, number, state, file_path, size)
                     VALUES (?1, ?2, 'done', ?3, ?4)
                     ON CONFLICT (mal_id, number) DO UPDATE SET
                         state = 'done',
                         file_path = excluded.file_path,
                         size = excluded.size",
                    rusqlite::params![context.mal_id, ep_num, final_dest, file_size],
                )?;
                Ok(())
            });
        }

        if primary_dest.is_empty() {
            primary_dest = show_dir.display().to_string();
        }

        Ok(FinalizeOutcome::Done(FinalizeSuccess {
            destination_path: primary_dest,
        }))
    }
}
