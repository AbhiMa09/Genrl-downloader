use std::sync::Arc;
use std::time::Duration;

use futures_util::future::join_all;
use tracing::warn;

use super::dedupe::dedupe_releases;
use super::indexer_trait::Indexer;
use super::redact::redact_secrets;
use super::types::{MediaKind, Release, SearchQuery};

pub struct IndexerEntry {
    pub indexer: Arc<dyn Indexer>,
    pub kinds: Vec<MediaKind>,
    pub enabled: bool,
}

#[derive(Default)]
pub struct IndexerRegistry {
    entries: Vec<IndexerEntry>,
}

impl IndexerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, indexer: Arc<dyn Indexer>, kinds: Vec<MediaKind>) {
        self.entries.push(IndexerEntry {
            indexer,
            kinds,
            enabled: true,
        });
    }

    pub fn add_entry(&mut self, indexer: Arc<dyn Indexer>, kinds: Vec<MediaKind>, enabled: bool) {
        self.entries.push(IndexerEntry {
            indexer,
            kinds,
            enabled,
        });
    }

    /// Search all indexers configured for the given MediaKind in parallel with a 10s timeout.
    pub async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<Release>, super::types::IndexerError> {
        self.search_with_timeout(query, Duration::from_secs(10))
            .await
    }

    /// Search with a specific per-indexer timeout (used in tests to run fast).
    pub async fn search_with_timeout(
        &self,
        query: &SearchQuery,
        per_indexer_timeout: Duration,
    ) -> Result<Vec<Release>, super::types::IndexerError> {
        let matching: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.enabled && e.kinds.contains(&query.kind))
            .map(|e| e.indexer.clone())
            .collect();

        if matching.is_empty() {
            return Ok(Vec::new());
        }

        let num_matching = matching.len();
        let tasks = matching.into_iter().map(|idx| {
            let q = query.clone();
            async move {
                let id = idx.id().to_string();
                let fut = idx.search(&q);
                match tokio::time::timeout(per_indexer_timeout, fut).await {
                    Ok(Ok(releases)) => Ok(releases),
                    Ok(Err(err)) => {
                        let msg = redact_secrets(&err.to_string());
                        warn!("indexer {id} search error: {msg}");
                        Err(err)
                    }
                    Err(_) => {
                        warn!(
                            "indexer {id} search timed out after {:?}",
                            per_indexer_timeout
                        );
                        Err(super::types::IndexerError::Timeout)
                    }
                }
            }
        });

        let results: Vec<Result<Vec<Release>, super::types::IndexerError>> = join_all(tasks).await;
        let mut all_releases = Vec::new();
        let mut last_err = None;
        let mut err_count = 0;

        for res in results {
            match res {
                Ok(releases) => all_releases.extend(releases),
                Err(e) => {
                    last_err = Some(e);
                    err_count += 1;
                }
            }
        }

        if err_count == num_matching {
            if let Some(err) = last_err {
                return Err(err);
            }
        }

        Ok(dedupe_releases(all_releases))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::types::{Caps, DownloadSource, IndexerError};
    use async_trait::async_trait;

    struct FakeOkIndexer;
    #[async_trait]
    impl Indexer for FakeOkIndexer {
        fn id(&self) -> &str {
            "ok_idx"
        }
        fn caps(&self) -> &Caps {
            static CAPS: Caps = Caps {
                tv_available: true,
                movie_available: true,
                tv_params: Vec::new(),
                movie_params: Vec::new(),
                categories: Vec::new(),
            };
            &CAPS
        }
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<Release>, IndexerError> {
            Ok(vec![Release {
                title: "Dune 2021 1080p".into(),
                indexer_id: "ok_idx".into(),
                size_bytes: 4_000_000_000,
                seeders: 50,
                leechers: 5,
                download: DownloadSource::Magnet("magnet:?xt=test1".into()),
                info_hash: Some("HASH111".into()),
                categories: vec![2000],
                native_category: None,
                published_date: None,
                trusted: false,
                remake: false,
            }])
        }
    }

    struct FakeSlowIndexer;
    #[async_trait]
    impl Indexer for FakeSlowIndexer {
        fn id(&self) -> &str {
            "slow_idx"
        }
        fn caps(&self) -> &Caps {
            static CAPS: Caps = Caps {
                tv_available: true,
                movie_available: true,
                tv_params: Vec::new(),
                movie_params: Vec::new(),
                categories: Vec::new(),
            };
            &CAPS
        }
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<Release>, IndexerError> {
            tokio::time::sleep(Duration::from_millis(500)).await;
            Ok(vec![Release {
                title: "Dune 2021 Slow 1080p".into(),
                indexer_id: "slow_idx".into(),
                size_bytes: 4_000_000_000,
                seeders: 10,
                leechers: 1,
                download: DownloadSource::Magnet("magnet:?xt=test2".into()),
                info_hash: Some("HASH222".into()),
                categories: vec![2000],
                native_category: None,
                published_date: None,
                trusted: false,
                remake: false,
            }])
        }
    }

    struct FakeFailingIndexer;
    #[async_trait]
    impl Indexer for FakeFailingIndexer {
        fn id(&self) -> &str {
            "fail_idx"
        }
        fn caps(&self) -> &Caps {
            static CAPS: Caps = Caps {
                tv_available: true,
                movie_available: true,
                tv_params: Vec::new(),
                movie_params: Vec::new(),
                categories: Vec::new(),
            };
            &CAPS
        }
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<Release>, IndexerError> {
            Err(IndexerError::Http(
                "Connection reset by peer?apikey=SECRET123".into(),
            ))
        }
    }

    #[tokio::test]
    async fn parallel_search_with_ok_slow_and_failing_indexers() {
        let mut registry = IndexerRegistry::new();
        registry.add(Arc::new(FakeOkIndexer), vec![MediaKind::Movie]);
        registry.add(Arc::new(FakeSlowIndexer), vec![MediaKind::Movie]);
        registry.add(Arc::new(FakeFailingIndexer), vec![MediaKind::Movie]);

        let query = SearchQuery::new("Dune", MediaKind::Movie);
        // Using a 50ms timeout so the slow 500ms indexer times out cleanly
        let results = registry
            .search_with_timeout(&query, Duration::from_millis(50))
            .await
            .expect("should succeed because ok_idx succeeded");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].indexer_id, "ok_idx");
        assert_eq!(results[0].title, "Dune 2021 1080p");
    }

    #[tokio::test]
    async fn all_failing_indexers_returns_error() {
        let mut registry = IndexerRegistry::new();
        registry.add(Arc::new(FakeFailingIndexer), vec![MediaKind::Movie]);

        let query = SearchQuery::new("Dune", MediaKind::Movie);
        let result = registry
            .search_with_timeout(&query, Duration::from_millis(50))
            .await;

        assert!(result.is_err(), "must return error when all indexers fail");
    }

    #[tokio::test]
    async fn disabled_indexer_is_skipped_by_search() {
        let mut registry = IndexerRegistry::new();
        // Add failing indexer as disabled
        registry.add_entry(Arc::new(FakeFailingIndexer), vec![MediaKind::Movie], false);
        // Add ok indexer as enabled
        registry.add_entry(Arc::new(FakeOkIndexer), vec![MediaKind::Movie], true);

        let query = SearchQuery::new("Dune", MediaKind::Movie);
        let results = registry
            .search_with_timeout(&query, Duration::from_millis(50))
            .await
            .expect("should succeed without error because failing indexer was disabled");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].indexer_id, "ok_idx");

        // Now test when the only indexer matching is disabled -> returns empty list immediately
        let mut disabled_only = IndexerRegistry::new();
        disabled_only.add_entry(Arc::new(FakeFailingIndexer), vec![MediaKind::Movie], false);
        let res = disabled_only
            .search_with_timeout(&query, Duration::from_millis(50))
            .await
            .expect("should return empty list when all matching indexers are disabled");
        assert!(res.is_empty());
    }
}
