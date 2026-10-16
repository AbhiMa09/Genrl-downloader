use std::path::Path;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::Gate;

use super::models::{
    clean_plain_text, clean_single_line, JikanFullData, MalApiAnimeDetailResponse,
};
use super::poster::download_poster_to_file;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataOutcome {
    Updated {
        mal_id: i64,
        synopsis: String,
        poster_path: Option<String>,
    },
    Cached {
        mal_id: i64,
    },
    PendingRetry {
        mal_id: i64,
        next_run_at: i64,
        error: String,
    },
    Failed {
        mal_id: i64,
        error: String,
    },
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_jikan_metadata(
    db: &Db,
    gate: &Gate,
    data_dir: &Path,
    clock: &dyn Clock,
    mal_id: i64,
    _current_title: &str,
    current_english: Option<&str>,
    _folder_path: Option<&str>,
    existing_poster: Option<&str>,
    data: &JikanFullData,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let clean_syn = data
        .synopsis
        .as_deref()
        .map(clean_plain_text)
        .unwrap_or_default();

    // Determine poster URL (large JPG -> JPG -> large WebP -> WebP) or existing remote URL
    let poster_url = data
        .images
        .as_ref()
        .and_then(|imgs| {
            imgs.jpg
                .as_ref()
                .and_then(|j| j.large_image_url.clone().or_else(|| j.image_url.clone()))
                .or_else(|| {
                    imgs.webp
                        .as_ref()
                        .and_then(|w| w.large_image_url.clone().or_else(|| w.image_url.clone()))
                })
        })
        .or_else(|| {
            existing_poster
                .filter(|p| p.starts_with("http"))
                .map(|s| s.to_string())
        });

    let local_poster_path = match poster_url {
        Some(ref url) => download_poster_to_file(gate, data_dir, mal_id, url)
            .await
            .map(|p| p.display().to_string()),
        None => None,
    };

    let start_year = data.year.or_else(|| {
        data.aired.as_ref().and_then(|a| {
            a.prop
                .as_ref()
                .and_then(|p| p.from.as_ref().and_then(|f| f.year))
        })
    });

    let english_title = data.title_english.as_deref().or(current_english);

    db.with(|c| {
        c.execute(
            "UPDATE shows SET
                synopsis = ?1,
                poster_path = COALESCE(?2, poster_path),
                score = COALESCE(?3, score),
                type = COALESCE(?4, type),
                total_episodes = COALESCE(?5, total_episodes),
                start_year = COALESCE(?6, start_year),
                english_title = COALESCE(?7, english_title),
                metadata_status = 'ready',
                metadata_updated_at = ?8,
                metadata_next_retry_at = NULL,
                metadata_retry_count = 0,
                metadata_error = NULL
             WHERE mal_id = ?9",
            rusqlite::params![
                clean_syn,
                local_poster_path,
                data.score,
                data.show_type.as_deref().map(|t| t.to_lowercase()),
                data.episodes,
                start_year,
                english_title,
                now,
                mal_id,
            ],
        )?;

        // Also add Japanese and English titles to show_names if present
        if let Some(ref ja) = data.title_japanese {
            if !ja.trim().is_empty() {
                let norm = crate::names::normalize_alias(ja);
                let _ = c.execute(
                    "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                     VALUES (?1, ?2, ?3, 'jikan', 3)
                     ON CONFLICT (mal_id, normalized) DO NOTHING",
                    rusqlite::params![mal_id, clean_single_line(ja), norm],
                );
            }
        }
        if let Some(ref en) = data.title_english {
            if !en.trim().is_empty() {
                let norm = crate::names::normalize_alias(en);
                let _ = c.execute(
                    "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                     VALUES (?1, ?2, ?3, 'jikan', 2)
                     ON CONFLICT (mal_id, normalized) DO NOTHING",
                    rusqlite::params![mal_id, clean_single_line(en), norm],
                );
            }
        }

        Ok(())
    })?;

    // Rewrite synopsis.txt and movie.nfo/tvshow.nfo across all existing folders of this show
    let _ = crate::library::rewrite_show_folder_metadata(db, mal_id);

    let _ = log.info(
        None,
        Some(mal_id),
        format!("Metadata updated for \"{}\" (mal_id: {mal_id})", data.title),
    );

    Ok(MetadataOutcome::Updated {
        mal_id,
        synopsis: clean_syn,
        poster_path: local_poster_path,
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn apply_mal_metadata(
    db: &Db,
    gate: &Gate,
    data_dir: &Path,
    clock: &dyn Clock,
    mal_id: i64,
    _current_title: &str,
    current_english: Option<&str>,
    _folder_path: Option<&str>,
    existing_poster: Option<&str>,
    data: &MalApiAnimeDetailResponse,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let clean_syn = data
        .synopsis
        .as_deref()
        .map(clean_plain_text)
        .unwrap_or_default();

    let poster_url = data
        .main_picture
        .as_ref()
        .and_then(|p| p.large.clone().or_else(|| p.medium.clone()))
        .or_else(|| {
            existing_poster
                .filter(|p| p.starts_with("http"))
                .map(|s| s.to_string())
        });

    let local_poster_path = match poster_url {
        Some(ref url) => download_poster_to_file(gate, data_dir, mal_id, url)
            .await
            .map(|p| p.display().to_string()),
        None => None,
    };

    let start_year = data.start_season.as_ref().and_then(|s| s.year);
    let en_title = data
        .alternative_titles
        .as_ref()
        .and_then(|a| a.en.clone())
        .or_else(|| current_english.map(|s| s.to_string()));
    let ja_title = data.alternative_titles.as_ref().and_then(|a| a.ja.clone());

    db.with(|c| {
        c.execute(
            "UPDATE shows SET
                synopsis = ?1,
                poster_path = COALESCE(?2, poster_path),
                score = COALESCE(?3, score),
                type = COALESCE(?4, type),
                total_episodes = COALESCE(?5, total_episodes),
                start_year = COALESCE(?6, start_year),
                english_title = COALESCE(?7, english_title),
                metadata_status = 'ready',
                metadata_updated_at = ?8,
                metadata_next_retry_at = NULL,
                metadata_retry_count = 0,
                metadata_error = NULL
             WHERE mal_id = ?9",
            rusqlite::params![
                clean_syn,
                local_poster_path,
                data.mean,
                data.media_type.as_deref().map(|t| t.to_lowercase()),
                data.num_episodes,
                start_year,
                en_title,
                now,
                mal_id,
            ],
        )?;

        if let Some(ref ja) = ja_title {
            if !ja.trim().is_empty() {
                let norm = crate::names::normalize_alias(ja);
                let _ = c.execute(
                    "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                     VALUES (?1, ?2, ?3, 'mal', 3)
                     ON CONFLICT (mal_id, normalized) DO NOTHING",
                    rusqlite::params![mal_id, clean_single_line(ja), norm],
                );
            }
        }
        if let Some(ref en) = en_title {
            if !en.trim().is_empty() {
                let norm = crate::names::normalize_alias(en);
                let _ = c.execute(
                    "INSERT INTO show_names (mal_id, name, normalized, source, priority)
                     VALUES (?1, ?2, ?3, 'mal', 2)
                     ON CONFLICT (mal_id, normalized) DO NOTHING",
                    rusqlite::params![mal_id, clean_single_line(en), norm],
                );
            }
        }

        Ok(())
    })?;

    // Rewrite synopsis.txt and movie.nfo/tvshow.nfo across all existing folders of this show
    let _ = crate::library::rewrite_show_folder_metadata(db, mal_id);

    let _ = log.info(
        None,
        Some(mal_id),
        format!(
            "Metadata updated from MAL API fallback for \"{}\" (mal_id: {mal_id})",
            data.title
        ),
    );

    Ok(MetadataOutcome::Updated {
        mal_id,
        synopsis: clean_syn,
        poster_path: local_poster_path,
    })
}
