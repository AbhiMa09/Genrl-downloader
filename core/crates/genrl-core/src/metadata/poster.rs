use std::fs;
use std::path::{Path, PathBuf};

use crate::http::{Gate, GetOptions, Service};

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

    match gate
        .get_with(Service::Images, url, &GetOptions::default())
        .await
    {
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
