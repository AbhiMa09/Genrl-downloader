use std::sync::Arc;

use async_trait::async_trait;

use super::indexer_trait::Indexer;
use super::types::{Caps, DownloadSource, IndexerError, MediaKind, Release, SearchQuery};
use crate::http::Gate;
use crate::nyaa;
use crate::settings::Settings;

/// Indexer adapter wrapping the existing Nyaa search engine.
pub struct NyaaAdapter {
    gate: Arc<Gate>,
    settings: Settings,
    caps: Caps,
}

impl NyaaAdapter {
    pub fn new(gate: Arc<Gate>, settings: Settings) -> Self {
        let caps = Caps {
            tv_available: true,
            movie_available: true,
            tv_params: vec!["q".into()],
            movie_params: vec!["q".into()],
            categories: Vec::new(),
        };
        Self {
            gate,
            settings,
            caps,
        }
    }
}

#[async_trait]
impl Indexer for NyaaAdapter {
    fn id(&self) -> &str {
        "nyaa"
    }

    fn caps(&self) -> &Caps {
        &self.caps
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Release>, IndexerError> {
        // Nyaa is dedicated to anime
        if query.kind != MediaKind::Anime {
            return Ok(Vec::new());
        }

        let mut nyaa_query = nyaa::SearchQuery::new(&query.text);
        nyaa_query.min_seeders = 1;
        nyaa_query.limit = 1000;

        let items = nyaa::search(&self.gate, &self.settings, &nyaa_query).await?;

        let releases = items
            .into_iter()
            .map(|item| Release {
                title: item.title.clone(),
                indexer_id: "nyaa".to_string(),
                size_bytes: item.size_bytes,
                seeders: item.seeders,
                leechers: item.leechers,
                download: DownloadSource::Magnet(item.magnet()),
                info_hash: Some(item.info_hash),
                categories: Vec::new(),
                native_category: Some(item.category_id),
                published_date: Some(item.published_at.to_string()),
                trusted: item.trusted,
                remake: item.remake,
            })
            .collect();

        Ok(releases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nyaa_adapter_id_and_caps() {
        let db = crate::Db::open_in_memory().unwrap();
        let gate = Arc::new(Gate::new(db).unwrap());
        let settings = Settings::default();
        let adapter = NyaaAdapter::new(gate, settings);
        assert_eq!(adapter.id(), "nyaa");
        assert!(adapter.caps().tv_available);
        assert!(adapter.caps().movie_available);
    }

    #[tokio::test]
    async fn nyaa_adapter_rejects_non_anime() {
        let db = crate::Db::open_in_memory().unwrap();
        let gate = Arc::new(Gate::new(db).unwrap());
        let settings = Settings::default();
        let adapter = NyaaAdapter::new(gate, settings);

        let query = SearchQuery::new("Inception 2010", MediaKind::Movie);
        let results = adapter.search(&query).await.unwrap();
        assert!(
            results.is_empty(),
            "Nyaa adapter must ignore non-anime kinds"
        );
    }
}
