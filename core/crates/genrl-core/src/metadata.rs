//! Metadata ingestion module: Jikan /v4/anime/{id}/full, MAL API fallback,
//! 7-day caching, local poster downloads, plain text sanitization, and retry with backoff.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::{Gate, GetOptions, Service};
use crate::settings::{self, Settings};

// ---------------------------------------------------------------------------
// Jikan API Wire Models (/v4/anime/{id}/full)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullResponse {
    pub data: JikanFullData,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullData {
    pub mal_id: i64,
    pub title: String,
    #[serde(default)]
    pub title_english: Option<String>,
    #[serde(default)]
    pub title_japanese: Option<String>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub score: Option<f64>,
    #[serde(rename = "type", default)]
    pub show_type: Option<String>,
    #[serde(default)]
    pub episodes: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub aired: Option<JikanFullAired>,
    #[serde(default)]
    pub images: Option<JikanFullImages>,
    #[serde(default)]
    pub genres: Vec<JikanNamedItem>,
    #[serde(default)]
    pub studios: Vec<JikanNamedItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullAired {
    #[serde(default)]
    pub prop: Option<JikanAiredProp>,
    #[serde(default)]
    pub string: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAiredProp {
    #[serde(default)]
    pub from: Option<JikanAiredDate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanAiredDate {
    #[serde(default)]
    pub year: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanFullImages {
    #[serde(default)]
    pub jpg: Option<JikanImageSet>,
    #[serde(default)]
    pub webp: Option<JikanImageSet>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanImageSet {
    #[serde(default)]
    pub large_image_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JikanNamedItem {
    pub name: String,
}

// ---------------------------------------------------------------------------
// MAL API Fallback Wire Models (/v2/anime/{id})
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiAnimeDetailResponse {
    pub id: i64,
    pub title: String,
    #[serde(default)]
    pub alternative_titles: Option<MalApiAlternativeTitles>,
    #[serde(default)]
    pub main_picture: Option<MalApiPicture>,
    #[serde(default)]
    pub synopsis: Option<String>,
    #[serde(default)]
    pub mean: Option<f64>,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub num_episodes: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub start_season: Option<MalApiSeason>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiAlternativeTitles {
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub ja: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiPicture {
    #[serde(default)]
    pub large: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MalApiSeason {
    #[serde(default)]
    pub year: Option<i64>,
}

// ---------------------------------------------------------------------------
// Sanitization Helpers
// ---------------------------------------------------------------------------

/// Strip control characters and ANSI escape sequences from text.
/// Preserves standard newlines (\n, \r) and tabs (\t).
pub fn clean_plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;

    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
            continue;
        }
        // ASCII control chars below 32 except \n (10), \r (13), \t (9)
        if (c as u32) < 32 && c != '\n' && c != '\r' && c != '\t' {
            continue;
        }
        // DEL (0x7f) and zero-width spaces/marks
        if c == '\x7f' || c == '\u{feff}' || c == '\u{200b}' {
            continue;
        }
        out.push(c);
    }

    out.trim().to_string()
}

/// Strip all control characters AND newlines for single-line headers or filenames.
pub fn clean_single_line(s: &str) -> String {
    clean_plain_text(s)
        .replace('\r', " ")
        .replace('\n', " ")
        .replace('\t', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calculate backoff in seconds based on retry attempt count.
pub fn calculate_backoff(retry_count: u32) -> i64 {
    match retry_count {
        0 | 1 => 30,
        2 => 60,
        3 => 120,
        4 => 300,
        5 => 600,
        _ => 1800,
    }
}

pub fn placeholder_synopsis() -> &'static str {
    "[Synopsis loading... could not fetch metadata, will retry]"
}

// ---------------------------------------------------------------------------
// Poster Management
// ---------------------------------------------------------------------------

/// Download and persist show poster locally to `{data_dir}/posters/{mal_id}.jpg`.
pub async fn download_poster_to_file(
    gate: &Gate,
    data_dir: &Path,
    mal_id: i64,
    url: &str,
) -> Option<PathBuf> {
    let poster_dir = data_dir.join("posters");
    let _ = fs::create_dir_all(&poster_dir);
    let dst = poster_dir.join(format!("{mal_id}.jpg"));

    // Check if valid existing file already exists
    if dst.exists() {
        if let Ok(meta) = fs::metadata(&dst) {
            if meta.len() > 0 {
                return Some(dst);
            }
        }
    }

    match gate.get_with(Service::Images, url, &GetOptions::default()).await {
        Ok(fetched) => {
            if !fetched.body.is_empty() && fs::write(&dst, &fetched.body).is_ok() {
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

// ---------------------------------------------------------------------------
// Queue Stale Metadata
// ---------------------------------------------------------------------------

/// Queue all shows missing synopsis/poster or with stale metadata (> 7 days).
pub fn queue_stale_metadata(db: &Db, now: i64) -> Result<usize> {
    let stale_threshold = now - (7 * 86400);
    let count: usize = db.with(|c| {
        let changed = c.execute(
            "UPDATE shows SET
                metadata_status = 'pending',
                metadata_next_retry_at = ?1
             WHERE (metadata_status != 'ready'
                    OR metadata_updated_at IS NULL
                    OR metadata_updated_at < ?2
                    OR synopsis IS NULL
                    OR trim(synopsis) = ''
                    OR poster_path IS NULL
                    OR trim(poster_path) = ''
                    OR poster_path LIKE 'http%')
               AND (metadata_next_retry_at IS NULL OR metadata_next_retry_at <= ?1)",
            rusqlite::params![now, stale_threshold],
        )?;
        Ok(changed)
    })?;
    Ok(count)
}

// ---------------------------------------------------------------------------
// Fetch Show Metadata (Jikan + MAL Fallback)
// ---------------------------------------------------------------------------

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

/// Fetch metadata for a show through the Gate.
/// 1. Check 7-day cache: if fresh and !force, returns Cached.
/// 2. If backoff is active and !force, returns PendingRetry.
/// 3. Queries Jikan /v4/anime/{id}/full.
/// 4. If Jikan has no data or fails, falls back to MAL API when connected.
/// 5. On failure, saves placeholder, sets metadata_status = 'pending' with backoff.
pub async fn fetch_show_metadata(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    data_dir: &Path,
    mal_id: i64,
    force: bool,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    // 1. Read current show status
    let show_row = db.with(|c| {
        c.query_row(
            "SELECT title, english_title, synopsis, poster_path,
                    metadata_status, metadata_updated_at, metadata_next_retry_at, metadata_retry_count
             FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<i64>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                ))
            },
        )
        .map_err(Into::into)
    });

    let (title, english_title, existing_syn, existing_poster, meta_status, updated_at, next_retry, retry_count) =
        match show_row {
            Ok(data) => data,
            Err(e) => return Err(e),
        };

    let meta_status_str = meta_status.unwrap_or_else(|| "pending".into());
    let retry_cnt = retry_count.unwrap_or(0) as u32;

    // 2. Cache Check: 7 days
    if !force && meta_status_str == "ready" {
        if let Some(ts) = updated_at {
            if now - ts < 7 * 86400 {
                let syn_ok = existing_syn.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                let poster_ok = existing_poster.as_deref().map(|p| !p.trim().is_empty()).unwrap_or(false);
                if syn_ok && poster_ok {
                    return Ok(MetadataOutcome::Cached { mal_id });
                }
            }
        }
    }

    // 3. Backoff Check: if too early to retry and not force
    if !force {
        if let Some(retry_at) = next_retry {
            if retry_at > now {
                return Ok(MetadataOutcome::PendingRetry {
                    mal_id,
                    next_run_at: retry_at,
                    error: "Retry scheduled with backoff".to_string(),
                });
            }
        }
    }

    // Set status to fetching
    let _ = db.with(|c| {
        c.execute(
            "UPDATE shows SET metadata_status = 'fetching' WHERE mal_id = ?1",
            [mal_id],
        )?;
        Ok(())
    });

    // 4. Try Jikan /v4/anime/{mal_id}/full
    let raw_base = settings.jikan_base_url.trim_end_matches('/');
    let jikan_base = if raw_base.ends_with("/v4") {
        raw_base.to_string()
    } else {
        format!("{raw_base}/v4")
    };
    let jikan_url = format!("{jikan_base}/anime/{mal_id}/full");

    let jikan_opts = GetOptions {
        ttl: Some(Duration::from_secs(7 * 86400)),
        skip_cache: force,
        ..Default::default()
    };

    let mut jikan_fetched_data: Option<JikanFullData> = None;
    let mut last_error: Option<String> = None;

    match gate.get_with(Service::Jikan, &jikan_url, &jikan_opts).await {
        Ok(fetched) => {
            match fetched.json::<JikanFullResponse>() {
                Ok(resp) => {
                    if let Some(ref syn) = resp.data.synopsis {
                        if !syn.trim().is_empty() {
                            jikan_fetched_data = Some(resp.data);
                        }
                    } else {
                        // Empty synopsis in Jikan, will attempt fallback
                        jikan_fetched_data = Some(resp.data);
                    }
                }
                Err(err) => {
                    last_error = Some(format!("Failed to parse Jikan JSON: {err}"));
                }
            }
        }
        Err(err) => {
            last_error = Some(format!("Jikan request failed: {err}"));
        }
    }

    // If Jikan succeeded and has synopsis, use it
    if let Some(data) = jikan_fetched_data.as_ref() {
        if let Some(syn) = data.synopsis.as_ref() {
            if !syn.trim().is_empty() {
                return apply_jikan_metadata(db, gate, data_dir, clock, mal_id, &title, english_title.as_deref(), data).await;
            }
        }
    }

    // 5. Fallback to MAL API if Jikan has no synopsis or failed
    if let Ok(Some(access_token)) = settings::read_state(db, "mal_oauth_access_token") {
        if !access_token.trim().is_empty() {
            let mal_base = settings.mal_base_url.trim_end_matches('/');
            let mal_url = format!(
                "{mal_base}/anime/{mal_id}?fields=synopsis,main_picture,mean,media_type,num_episodes,start_season,alternative_titles,genres,status"
            );
            let mal_opts = GetOptions {
                ttl: Some(Duration::from_secs(7 * 86400)),
                headers: vec![("Authorization".into(), format!("Bearer {access_token}"))],
                skip_cache: force,
                ..Default::default()
            };

            match gate.get_with(Service::Mal, &mal_url, &mal_opts).await {
                Ok(fetched) => {
                    if let Ok(mal_data) = fetched.json::<MalApiAnimeDetailResponse>() {
                        if let Some(ref syn) = mal_data.synopsis {
                            if !syn.trim().is_empty() {
                                return apply_mal_metadata(db, gate, data_dir, clock, mal_id, &title, &mal_data).await;
                            }
                        }
                    }
                }
                Err(err) => {
                    tracing::warn!(mal_id, "MAL API fallback failed: {err}");
                }
            }
        }
    }

    // If Jikan had metadata (e.g. title/score/images) but no synopsis, apply what it had
    if let Some(data) = jikan_fetched_data {
        return apply_jikan_metadata(db, gate, data_dir, clock, mal_id, &title, english_title.as_deref(), &data).await;
    }

    // 6. Failure handling: store placeholder, set metadata_status = pending with next_run_at and retry with backoff
    let new_retry_count = retry_cnt + 1;
    let backoff_secs = calculate_backoff(new_retry_count);
    let next_run_at = now + backoff_secs;
    let err_str = last_error.unwrap_or_else(|| "Failed to fetch metadata from Jikan or MAL".to_string());

    let _ = db.with(|c| {
        c.execute(
            "UPDATE shows SET
                synopsis = COALESCE(NULLIF(synopsis, ''), ?1),
                metadata_status = 'pending',
                metadata_next_retry_at = ?2,
                metadata_retry_count = ?3,
                metadata_error = ?4
             WHERE mal_id = ?5",
            rusqlite::params![
                placeholder_synopsis(),
                next_run_at,
                new_retry_count,
                err_str,
                mal_id,
            ],
        )?;
        Ok(())
    });

    let _ = log.warn(
        None,
        Some(mal_id),
        format!("Metadata fetch failed for show #{mal_id}: {err_str}. Retrying in {backoff_secs}s."),
    );

    Ok(MetadataOutcome::PendingRetry {
        mal_id,
        next_run_at,
        error: err_str,
    })
}

// ---------------------------------------------------------------------------
// Apply Metadata Helpers
// ---------------------------------------------------------------------------

async fn apply_jikan_metadata(
    db: &Db,
    gate: &Gate,
    data_dir: &Path,
    clock: &dyn Clock,
    mal_id: i64,
    _current_title: &str,
    current_english: Option<&str>,
    data: &JikanFullData,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let clean_syn = data.synopsis.as_deref().map(clean_plain_text).unwrap_or_default();

    // Determine poster URL (large JPG -> JPG -> large WebP -> WebP)
    let poster_url = data.images.as_ref().and_then(|imgs| {
        imgs.jpg.as_ref().and_then(|j| j.large_image_url.clone().or_else(|| j.image_url.clone()))
            .or_else(|| imgs.webp.as_ref().and_then(|w| w.large_image_url.clone().or_else(|| w.image_url.clone())))
    });

    let local_poster_path = match poster_url {
        Some(ref url) => download_poster_to_file(gate, data_dir, mal_id, url)
            .await
            .map(|p| p.display().to_string()),
        None => None,
    };

    let start_year = data.year.or_else(|| {
        data.aired.as_ref().and_then(|a| a.prop.as_ref().and_then(|p| p.from.as_ref().and_then(|f| f.year)))
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

async fn apply_mal_metadata(
    db: &Db,
    gate: &Gate,
    data_dir: &Path,
    clock: &dyn Clock,
    mal_id: i64,
    _current_title: &str,
    data: &MalApiAnimeDetailResponse,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    let clean_syn = data.synopsis.as_deref().map(clean_plain_text).unwrap_or_default();

    let poster_url = data.main_picture.as_ref().and_then(|p| p.large.clone().or_else(|| p.medium.clone()));

    let local_poster_path = match poster_url {
        Some(ref url) => download_poster_to_file(gate, data_dir, mal_id, url)
            .await
            .map(|p| p.display().to_string()),
        None => None,
    };

    let start_year = data.start_season.as_ref().and_then(|s| s.year);
    let en_title = data.alternative_titles.as_ref().and_then(|a| a.en.clone());
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

    let _ = log.info(
        None,
        Some(mal_id),
        format!("Metadata updated from MAL API fallback for \"{}\" (mal_id: {mal_id})", data.title),
    );

    Ok(MetadataOutcome::Updated {
        mal_id,
        synopsis: clean_syn,
        poster_path: local_poster_path,
    })
}

// ---------------------------------------------------------------------------
// Batch Processor for Metadata Queue
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct MetadataBatchReport {
    pub processed: usize,
    pub updated: usize,
    pub failed: usize,
    pub cached: usize,
}

/// Process up to `limit` pending metadata jobs in the queue through Gate.
pub async fn process_metadata_queue(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    data_dir: &Path,
    limit: Option<usize>,
) -> Result<MetadataBatchReport> {
    let now = clock.now();
    let max_items = limit.unwrap_or(20);

    let pending_ids: Vec<i64> = db.with(|c| {
        let mut stmt = c.prepare(
            "SELECT mal_id FROM shows
             WHERE metadata_status = 'pending'
               AND (metadata_next_retry_at IS NULL OR metadata_next_retry_at <= ?1)
             ORDER BY metadata_retry_count ASC, mal_id ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![now, max_items as i64], |r| r.get(0))?;
        Ok(rows.filter_map(std::result::Result::ok).collect())
    })?;

    let mut report = MetadataBatchReport {
        processed: pending_ids.len(),
        ..Default::default()
    };

    for id in pending_ids {
        match fetch_show_metadata(db, gate, settings, clock, data_dir, id, false).await {
            Ok(MetadataOutcome::Updated { .. }) => report.updated += 1,
            Ok(MetadataOutcome::Cached { .. }) => report.cached += 1,
            Ok(MetadataOutcome::PendingRetry { .. }) | Ok(MetadataOutcome::Failed { .. }) => {
                report.failed += 1
            }
            Err(_) => report.failed += 1,
        }
    }

    Ok(report)
}
