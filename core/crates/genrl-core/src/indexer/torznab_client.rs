use std::sync::Arc;

use async_trait::async_trait;

use crate::db::Db;
use crate::indexer::fetch::HttpFetch;
use crate::indexer::indexer_trait::Indexer;
use crate::indexer::torznab_caps;
use crate::indexer::torznab_rss;
use crate::indexer::types::{Caps, IndexerError, MediaKind, Release, SearchQuery};

/// A Torznab indexer client connecting to a Prowlarr or generic Torznab endpoint.
pub struct TorznabClient {
    id: String,
    base_url: String,
    api_key: String,
    fetch: Arc<dyn HttpFetch + Send + Sync>,
    db: Db,
    caps: Caps,
}

impl TorznabClient {
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        fetch: Arc<dyn HttpFetch + Send + Sync>,
        db: Db,
    ) -> Self {
        let id_str = id.into();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let caps = torznab_caps::get_cached_caps(&db, &id_str, now)
            .ok()
            .flatten()
            .unwrap_or_default();
        Self {
            id: id_str,
            base_url: base_url.into(),
            api_key: api_key.into(),
            fetch,
            db,
            caps,
        }
    }

    pub fn with_caps(
        id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        fetch: Arc<dyn HttpFetch + Send + Sync>,
        db: Db,
        caps: Caps,
    ) -> Self {
        Self {
            id: id.into(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            fetch,
            db,
            caps,
        }
    }

    /// Read capabilities, utilizing the 24-hour SQLite cache or fetching live over HTTP.
    pub async fn read_caps(&self) -> Result<Caps, IndexerError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        if let Some(cached) = torznab_caps::get_cached_caps(&self.db, &self.id, now)? {
            return Ok(cached);
        }

        let breaker_key = format!("torznab:{}", self.id);
        let params = [("t", "caps"), ("apikey", &self.api_key)];
        let body = self
            .fetch
            .get_text(&breaker_key, &self.base_url, &params)
            .await?;
        let caps = torznab_caps::parse_caps(&body)?;
        let _ = torznab_caps::save_cached_caps(&self.db, &self.id, &body, now);
        Ok(caps)
    }
}

#[async_trait]
impl Indexer for TorznabClient {
    fn id(&self) -> &str {
        &self.id
    }

    fn caps(&self) -> &Caps {
        &self.caps
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Release>, IndexerError> {
        let caps = self.read_caps().await?;

        let mode = match query.kind {
            MediaKind::Movie => {
                if !caps.movie_available {
                    return Ok(Vec::new());
                }
                "movie"
            }
            MediaKind::Series => {
                if !caps.tv_available {
                    return Ok(Vec::new());
                }
                "tvsearch"
            }
            MediaKind::Anime => {
                if caps.tv_available {
                    "tvsearch"
                } else {
                    "search"
                }
            }
        };

        let mut params: Vec<(&str, String)> = Vec::new();
        params.push(("t", mode.to_string()));
        params.push(("extended", "1".to_string()));
        params.push(("apikey", self.api_key.clone()));

        match query.kind {
            MediaKind::Movie => {
                // Read whatever movie category IDs caps advertises (2000-2999)
                let movie_cats: Vec<String> = caps
                    .categories
                    .iter()
                    .filter(|&&c| (2000..=2999).contains(&c))
                    .map(|c| c.to_string())
                    .collect();
                if !movie_cats.is_empty() {
                    params.push(("cat", movie_cats.join(",")));
                }

                if let Some(imdb) = &query.imdb_id {
                    if caps.movie_params.iter().any(|p| p == "imdbid") {
                        params.push(("imdbid", imdb.clone()));
                    } else if let Some(tmdb) = query.tmdb_id {
                        if caps.movie_params.iter().any(|p| p == "tmdbid") {
                            params.push(("tmdbid", tmdb.to_string()));
                        } else {
                            params.push(("q", query.text.clone()));
                        }
                    } else {
                        params.push(("q", query.text.clone()));
                    }
                } else if let Some(tmdb) = query.tmdb_id {
                    if caps.movie_params.iter().any(|p| p == "tmdbid") {
                        params.push(("tmdbid", tmdb.to_string()));
                    } else {
                        params.push(("q", query.text.clone()));
                    }
                } else {
                    params.push(("q", query.text.clone()));
                }
            }
            MediaKind::Series => {
                // Read whatever TV category IDs caps advertises (5000-5999)
                let tv_cats: Vec<String> = caps
                    .categories
                    .iter()
                    .filter(|&&c| (5000..=5999).contains(&c))
                    .map(|c| c.to_string())
                    .collect();
                if !tv_cats.is_empty() {
                    params.push(("cat", tv_cats.join(",")));
                }

                if let Some(imdb) = &query.imdb_id {
                    if caps.tv_params.iter().any(|p| p == "imdbid") {
                        params.push(("imdbid", imdb.clone()));
                    } else if let Some(tvdb) = query.tvdb_id {
                        if caps.tv_params.iter().any(|p| p == "tvdbid") {
                            params.push(("tvdbid", tvdb.to_string()));
                        } else if let Some(tmdb) = query.tmdb_id {
                            if caps.tv_params.iter().any(|p| p == "tmdbid") {
                                params.push(("tmdbid", tmdb.to_string()));
                            } else {
                                params.push(("q", query.text.clone()));
                            }
                        } else {
                            params.push(("q", query.text.clone()));
                        }
                    } else {
                        params.push(("q", query.text.clone()));
                    }
                } else if let Some(tvdb) = query.tvdb_id {
                    if caps.tv_params.iter().any(|p| p == "tvdbid") {
                        params.push(("tvdbid", tvdb.to_string()));
                    } else if let Some(tmdb) = query.tmdb_id {
                        if caps.tv_params.iter().any(|p| p == "tmdbid") {
                            params.push(("tmdbid", tmdb.to_string()));
                        } else {
                            params.push(("q", query.text.clone()));
                        }
                    } else {
                        params.push(("q", query.text.clone()));
                    }
                } else if let Some(tmdb) = query.tmdb_id {
                    if caps.tv_params.iter().any(|p| p == "tmdbid") {
                        params.push(("tmdbid", tmdb.to_string()));
                    } else {
                        params.push(("q", query.text.clone()));
                    }
                } else {
                    params.push(("q", query.text.clone()));
                }

                if let Some(s) = query.season {
                    if caps.tv_params.iter().any(|p| p == "season") {
                        params.push(("season", s.to_string()));
                    }
                }
                if let Some(ep) = query.episode {
                    if caps.tv_params.iter().any(|p| p == "ep") {
                        let ep_str = if (ep.fract() - 0.0).abs() < f64::EPSILON {
                            format!("{}", ep as i64)
                        } else {
                            format!("{ep}")
                        };
                        params.push(("ep", ep_str));
                    }
                }
            }
            MediaKind::Anime => {
                // Torznab anime support: dynamically consume all IDs under 5000-5999
                let anime_cats: Vec<String> = caps
                    .categories
                    .iter()
                    .filter(|&&c| (5000..=5999).contains(&c))
                    .map(|c| c.to_string())
                    .collect();

                if !anime_cats.is_empty() {
                    params.push(("cat", anime_cats.join(",")));
                }
                params.push(("q", query.text.clone()));
            }
        }

        let ref_params: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let breaker_key = format!("torznab:{}", self.id);
        let body = self
            .fetch
            .get_text(&breaker_key, &self.base_url, &ref_params)
            .await?;

        torznab_rss::parse_rss(&self.id, &body)
    }
}
