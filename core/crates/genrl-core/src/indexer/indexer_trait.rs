use async_trait::async_trait;

use super::types::{Caps, IndexerError, Release, SearchQuery};

/// Common interface for all indexers (Nyaa adapter, Torznab clients).
#[async_trait]
pub trait Indexer: Send + Sync {
    /// Unique identifier for this indexer instance.
    fn id(&self) -> &str;

    /// Supported capabilities.
    fn caps(&self) -> &Caps;

    /// Search the indexer with the given query.
    async fn search(&self, query: &SearchQuery) -> Result<Vec<Release>, IndexerError>;
}
