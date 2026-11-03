use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::clock::Clock;
use crate::db::Db;
use crate::error::Result;
use crate::events::EventLog;
use crate::http::{Gate, GetOptions, Service};
use crate::mal::{KeyringStore, MalClient};
use crate::settings::Settings;

use super::apply::{apply_jikan_metadata, apply_mal_metadata, MetadataOutcome};
use super::models::{
    calculate_backoff, JikanFullData, JikanFullResponse, MalApiAnimeDetailResponse,
};

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
    let store = KeyringStore;
    let mal = MalClient::new(gate, settings.clone(), &store, clock);
    fetch_show_metadata_with_client(db, gate, settings, clock, data_dir, mal_id, force, &mal).await
}

/// Share a MAL session across a batch; tests inject an in-memory token store.
#[allow(clippy::too_many_arguments)]
pub async fn fetch_show_metadata_with_client(
    db: &Db,
    gate: &Gate,
    settings: &Settings,
    clock: &dyn Clock,
    data_dir: &Path,
    mal_id: i64,
    force: bool,
    mal: &MalClient<'_>,
) -> Result<MetadataOutcome> {
    let now = clock.now();
    let log = EventLog::new(db.clone());

    // 1. Read current show status
    let show_row = db.with(|c| {
        c.query_row(
            "SELECT title, english_title, synopsis, poster_path,
                    metadata_status, metadata_updated_at, metadata_next_retry_at, metadata_retry_count,
                    folder_path
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
                    r.get::<_, Option<String>>(8)?,
                ))
            },
        )
        .map_err(Into::into)
    });

    let (
        title,
        english_title,
        existing_syn,
        existing_poster,
        meta_status,
        updated_at,
        next_retry,
        retry_count,
        folder_path,
    ) = match show_row {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    let meta_status_str = meta_status.unwrap_or_else(|| "pending".into());
    let retry_cnt = retry_count.unwrap_or(0) as u32;

    // Check if existing poster is a valid local file on disk
    let poster_is_local_file = existing_poster
        .as_deref()
        .map(|p| !p.trim().is_empty() && !p.starts_with("http") && Path::new(p).is_file())
        .unwrap_or(false);

    // 2. Cache Check: 7 days
    if !force && matches!(meta_status_str.as_str(), "ready" | "partial") {
        if let Some(ts) = updated_at {
            if now - ts < 7 * 86400 {
                let syn_ok = existing_syn
                    .as_deref()
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(false);
                if syn_ok && poster_is_local_file {
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
    let mut is_404 = false;
    let mut is_breaker_open = false;
    let mut minimum_retry_secs = 0i64;

    let service_retry_at =
        crate::settings::read_state_time(db, "jikan_retry_after_at")?.unwrap_or(0);
    let jikan_result = if service_retry_at > now {
        Err(crate::Error::RateLimited {
            service: "jikan".into(),
            retry_after: Some(Duration::from_secs((service_retry_at - now) as u64)),
        })
    } else {
        gate.get_with(Service::Jikan, &jikan_url, &jikan_opts).await
    };
    match jikan_result {
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
            minimum_retry_secs = match &err {
                crate::Error::RateLimited {
                    retry_after: Some(delay),
                    ..
                }
                | crate::Error::Status {
                    retry_after: Some(delay),
                    ..
                } => i64::try_from(delay.as_secs()).unwrap_or(i64::MAX),
                crate::Error::BreakerOpen { retry_in_secs, .. } => {
                    is_breaker_open = true;
                    i64::try_from(*retry_in_secs).unwrap_or(i64::MAX)
                }
                _ => 0,
            };
            if matches!(err, crate::Error::RateLimited { .. }) && minimum_retry_secs > 0 {
                let retry_at = now.saturating_add(minimum_retry_secs);
                db.with(|c| {
                    c.execute(
                        "INSERT INTO settings (key,value) VALUES ('state.jikan_retry_after_at',?1)
                        ON CONFLICT(key) DO UPDATE SET value=CAST(MAX(
                        CAST(settings.value AS INTEGER),CAST(excluded.value AS INTEGER)) AS TEXT)",
                        [retry_at.to_string()],
                    )?;
                    Ok(())
                })?;
            }
            if matches!(err, crate::Error::Status { status: 404, .. }) {
                is_404 = true;
                last_error = Some("Anime not found on Jikan (404)".to_string());
            } else {
                last_error = Some(format!("Jikan request failed: {err}"));
            }
        }
    }

    // If Jikan succeeded and has synopsis, use it
    if let Some(data) = jikan_fetched_data.as_ref() {
        if let Some(syn) = data.synopsis.as_ref() {
            if !syn.trim().is_empty() {
                return apply_jikan_metadata(
                    db,
                    gate,
                    data_dir,
                    clock,
                    mal_id,
                    &title,
                    english_title.as_deref(),
                    folder_path.as_deref(),
                    existing_poster.as_deref(),
                    data,
                )
                .await;
            }
        }
    }

    // 5. Fallback to MAL API if Jikan has no synopsis or failed
    // Access tokens remain in the MAL session, never in SQLite.
    let access_token = if settings.mal_client_id.trim().is_empty() {
        None
    } else {
        mal.access_token().await.ok()
    };
    if let Some(access_token) = access_token {
        if !access_token.trim().is_empty() {
            let mal_base = settings.mal_base_url.trim_end_matches('/');
            let mal_url = format!(
                "{mal_base}/anime/{mal_id}?fields=synopsis,main_picture,mean,media_type,num_episodes,start_date,start_season,alternative_titles,genres,status"
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
                        if mal_data.id == mal_id
                            && !mal_data.title.trim().is_empty()
                            && (mal_data.num_episodes.is_some()
                                || mal_data.status.is_some()
                                || mal_data
                                    .synopsis
                                    .as_ref()
                                    .is_some_and(|s| !s.trim().is_empty()))
                        {
                            return apply_mal_metadata(
                                db,
                                gate,
                                data_dir,
                                clock,
                                mal_id,
                                &title,
                                english_title.as_deref(),
                                folder_path.as_deref(),
                                existing_poster.as_deref(),
                                &mal_data,
                            )
                            .await;
                        }
                    }
                }
                Err(err) => {
                    if matches!(err, crate::Error::Status { status: 404, .. }) {
                        is_404 = true;
                    }
                    tracing::warn!(mal_id, "MAL API fallback failed: {err}");
                }
            }
        }
    }

    // If Jikan had metadata (e.g. title/score/images) but no synopsis, apply what it had
    if let Some(data) = jikan_fetched_data {
        return apply_jikan_metadata(
            db,
            gate,
            data_dir,
            clock,
            mal_id,
            &title,
            english_title.as_deref(),
            folder_path.as_deref(),
            existing_poster.as_deref(),
            &data,
        )
        .await;
    }

    // 6. Failure handling:
    // Distinguish permanent failure (404) from transient failure (429, 5xx, network):
    if is_404 {
        let err_str = "Anime not found (404)".to_string();
        let _ = db.with(|c| {
            c.execute(
                "UPDATE shows SET
                    synopsis = NULL,
                    metadata_status = 'failed',
                    metadata_next_retry_at = NULL,
                    metadata_retry_count = ?1,
                    metadata_error = ?2
                 WHERE mal_id = ?3",
                rusqlite::params![retry_cnt + 1, err_str, mal_id,],
            )?;
            Ok(())
        });

        // Write placeholder line to synopsis.txt if folder exists
        if let Some(ref folder) = folder_path {
            let folder_p = Path::new(folder);
            if folder_p.is_dir() {
                let syn_file = folder_p.join("synopsis.txt");
                let _ = fs::write(
                    syn_file,
                    format!("[Synopsis unavailable: show #{mal_id} returned 404 Not Found]\n"),
                );
            }
        }

        let _ = log.warn(
            None,
            Some(mal_id),
            format!("Metadata fetch failed permanently for show #{mal_id}: 404 Not Found"),
        );

        return Ok(MetadataOutcome::Failed {
            mal_id,
            error: err_str,
        });
    }

    // Circuit breaker open: halt without penalising the show or spamming warnings
    if is_breaker_open {
        let _ = db.with(|c| {
            c.execute(
                "UPDATE shows SET
                    metadata_status = 'pending',
                    metadata_error = 'Jikan circuit breaker open'
                 WHERE mal_id = ?1",
                [mal_id],
            )?;
            Ok(())
        });
        return Ok(MetadataOutcome::PendingRetry {
            mal_id,
            next_run_at: now,
            error: "Jikan circuit breaker open".to_string(),
        });
    }

    // Transient failure: 429, 5xx, network, etc.
    let new_retry_count = retry_cnt + 1;
    let backoff_secs = calculate_backoff(new_retry_count).max(minimum_retry_secs);
    let next_run_at = now.saturating_add(backoff_secs);
    let err_str =
        last_error.unwrap_or_else(|| "Failed to fetch metadata from Jikan or MAL".to_string());

    let _ = db.with(|c| {
        c.execute(
            "UPDATE shows SET
                synopsis = NULL,
                metadata_status = 'pending',
                metadata_next_retry_at = ?1,
                metadata_retry_count = ?2,
                metadata_error = ?3
             WHERE mal_id = ?4",
            rusqlite::params![next_run_at, new_retry_count, err_str, mal_id,],
        )?;
        Ok(())
    });

    // Write placeholder line to synopsis.txt if folder exists
    if let Some(ref folder) = folder_path {
        let folder_p = Path::new(folder);
        if folder_p.is_dir() {
            let syn_file = folder_p.join("synopsis.txt");
            let _ = fs::write(
                syn_file,
                format!("[Synopsis pending metadata sync for show #{mal_id}]\n"),
            );
        }
    }

    let _ = log.warn(
        None,
        Some(mal_id),
        format!(
            "Metadata fetch failed for show #{mal_id}: {err_str}. Retrying in {backoff_secs}s."
        ),
    );

    Ok(MetadataOutcome::PendingRetry {
        mal_id,
        next_run_at,
        error: err_str,
    })
}
