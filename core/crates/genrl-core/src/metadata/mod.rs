//! Metadata ingestion module: Jikan /v4/anime/{id}/full, MAL API fallback,
//! 7-day caching, local poster downloads, plain text sanitization, and retry with backoff.

pub mod apply;
pub mod batch;
pub mod fetch;
pub mod models;
pub mod poster;

pub use apply::*;
pub use batch::*;
pub use fetch::*;
pub use models::*;
pub use poster::*;

/// Download/search guards share the same resolution rule.
pub fn is_resolved(db: &crate::Db, mal_id: i64) -> crate::Result<bool> {
    let (status, title): (String, String) = db.with(|c| {
        Ok(c.query_row(
            "SELECT metadata_status, title FROM shows WHERE mal_id = ?1",
            [mal_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    })?;
    Ok(matches!(status.as_str(), "ready" | "partial")
        && !title.trim().is_empty()
        && !title.trim().eq_ignore_ascii_case("unknown"))
}
