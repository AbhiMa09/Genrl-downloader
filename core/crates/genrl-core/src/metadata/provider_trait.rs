use async_trait::async_trait;

use crate::error::Result;
use crate::indexer::types::MediaKind;
use crate::metadata::types::{EpisodeMetadata, ExternalIds, MediaMetadata};

/// Unified interface for movie, TV, or anime metadata lookup.
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Friendly name of provider (e.g. "tmdb").
    fn name(&self) -> &'static str;

    /// Search titles by query, optional media kind, and optional year.
    async fn search(
        &self,
        query: &str,
        kind: MediaKind,
        year: Option<u32>,
    ) -> Result<Vec<MediaMetadata>>;

    /// Retrieve external IDs (IMDb, TVDb, etc.) for a TMDB item.
    async fn get_external_ids(&self, tmdb_id: u32, kind: MediaKind) -> Result<ExternalIds>;

    /// Retrieve episode list for a TV season.
    async fn get_season_episodes(&self, tmdb_id: u32, season: u32) -> Result<Vec<EpisodeMetadata>>;
}
