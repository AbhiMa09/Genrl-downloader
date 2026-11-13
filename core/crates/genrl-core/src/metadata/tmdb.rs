use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::indexer::fetch::HttpFetch;
use crate::indexer::redact_secrets;
use crate::indexer::types::MediaKind;
use crate::metadata::provider_trait::MetadataProvider;
use crate::metadata::tmdb_models::{
    TmdbExternalIdsResponse, TmdbMovieItem, TmdbMovieSearchResponse, TmdbSeasonResponse,
    TmdbTvDetailsResponse, TmdbTvSearchResponse,
};
use crate::metadata::types::{EpisodeMetadata, ExternalIds, MediaMetadata};

const TMDB_BASE_URL: &str = "https://api.themoviedb.org/3";
const BREAKER_KEY: &str = "tmdb";

/// Client for The Movie Database (TMDB) API v3.
pub struct TmdbClient {
    api_key: String,
    base_url: String,
    fetch: Arc<dyn HttpFetch>,
}

impl TmdbClient {
    pub fn new(api_key: impl Into<String>, fetch: Arc<dyn HttpFetch>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: TMDB_BASE_URL.to_string(),
            fetch,
        }
    }

    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        fetch: Arc<dyn HttpFetch>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into(),
            fetch,
        }
    }

    /// Retrieve full TV show details including status and season summaries.
    pub async fn get_tv_details(&self, tmdb_id: u32) -> Result<TmdbTvDetailsResponse> {
        let url = format!("{}/tv/{}", self.base_url, tmdb_id);
        let params = [
            ("api_key", self.api_key.as_str()),
            ("include_adult", "false"),
            ("language", "en-US"),
        ];
        let body = self
            .fetch
            .get_text(BREAKER_KEY, &url, &params)
            .await
            .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

        serde_json::from_str(&body)
            .map_err(|e| Error::Other(format!("failed to parse TMDB tv details: {e}")))
    }

    /// Retrieve full movie details.
    pub async fn get_movie_details(&self, tmdb_id: u32) -> Result<TmdbMovieItem> {
        let url = format!("{}/movie/{}", self.base_url, tmdb_id);
        let params = [
            ("api_key", self.api_key.as_str()),
            ("include_adult", "false"),
            ("language", "en-US"),
        ];
        let body = self
            .fetch
            .get_text(BREAKER_KEY, &url, &params)
            .await
            .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

        serde_json::from_str(&body)
            .map_err(|e| Error::Other(format!("failed to parse TMDB movie details: {e}")))
    }
}

#[async_trait]
impl MetadataProvider for TmdbClient {
    fn name(&self) -> &'static str {
        "tmdb"
    }

    async fn search(
        &self,
        query: &str,
        kind: MediaKind,
        year: Option<u32>,
    ) -> Result<Vec<MediaMetadata>> {
        let year_str = year.map(|y| y.to_string());
        match kind {
            MediaKind::Movie => {
                let url = format!("{}/search/movie", self.base_url);
                let mut params = vec![
                    ("api_key", self.api_key.as_str()),
                    ("query", query),
                    ("language", "en-US"),
                    ("include_adult", "false"),
                ];
                if let Some(y) = year_str.as_deref() {
                    params.push(("year", y));
                }

                let body = self
                    .fetch
                    .get_text(BREAKER_KEY, &url, &params)
                    .await
                    .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

                let resp: TmdbMovieSearchResponse = serde_json::from_str(&body)
                    .map_err(|e| Error::Other(format!("failed to parse TMDB movie search: {e}")))?;

                let items = resp
                    .results
                    .into_iter()
                    .map(|item| {
                        let parsed_year = item
                            .release_date
                            .as_deref()
                            .and_then(|d| d.get(..4))
                            .and_then(|y| y.parse::<u32>().ok());
                        MediaMetadata {
                            kind: MediaKind::Movie,
                            tmdb_id: item.id,
                            external_ids: ExternalIds {
                                tmdb_id: Some(item.id),
                                ..Default::default()
                            },
                            title: item.title,
                            original_title: item.original_title,
                            year: parsed_year,
                            poster_path: item.poster_path,
                            status: None,
                            total_seasons: None,
                            total_episodes: None,
                            overview: item.overview,
                            raw_json: None,
                        }
                    })
                    .collect();

                Ok(items)
            }
            MediaKind::Series | MediaKind::Anime => {
                let url = format!("{}/search/tv", self.base_url);
                let mut params = vec![
                    ("api_key", self.api_key.as_str()),
                    ("query", query),
                    ("language", "en-US"),
                    ("include_adult", "false"),
                ];
                if let Some(y) = year_str.as_deref() {
                    params.push(("first_air_date_year", y));
                }

                let body = self
                    .fetch
                    .get_text(BREAKER_KEY, &url, &params)
                    .await
                    .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

                let resp: TmdbTvSearchResponse = serde_json::from_str(&body)
                    .map_err(|e| Error::Other(format!("failed to parse TMDB tv search: {e}")))?;

                let items = resp
                    .results
                    .into_iter()
                    .map(|item| {
                        let parsed_year = item
                            .first_air_date
                            .as_deref()
                            .and_then(|d| d.get(..4))
                            .and_then(|y| y.parse::<u32>().ok());
                        MediaMetadata {
                            kind,
                            tmdb_id: item.id,
                            external_ids: ExternalIds {
                                tmdb_id: Some(item.id),
                                ..Default::default()
                            },
                            title: item.name,
                            original_title: item.original_name,
                            year: parsed_year,
                            poster_path: item.poster_path,
                            status: None,
                            total_seasons: None,
                            total_episodes: None,
                            overview: item.overview,
                            raw_json: None,
                        }
                    })
                    .collect();

                Ok(items)
            }
        }
    }

    async fn get_external_ids(&self, tmdb_id: u32, kind: MediaKind) -> Result<ExternalIds> {
        let endpoint = match kind {
            MediaKind::Movie => "movie",
            MediaKind::Series | MediaKind::Anime => "tv",
        };
        let url = format!("{}/{}/{}/external_ids", self.base_url, endpoint, tmdb_id);
        let params = [
            ("api_key", self.api_key.as_str()),
            ("include_adult", "false"),
            ("language", "en-US"),
        ];

        let body = self
            .fetch
            .get_text(BREAKER_KEY, &url, &params)
            .await
            .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

        let resp: TmdbExternalIdsResponse = serde_json::from_str(&body)
            .map_err(|e| Error::Other(format!("failed to parse TMDB external ids: {e}")))?;

        Ok(ExternalIds {
            imdb_id: resp.imdb_id,
            tmdb_id: Some(resp.id),
            tvdb_id: resp.tvdb_id,
        })
    }

    async fn get_season_episodes(&self, tmdb_id: u32, season: u32) -> Result<Vec<EpisodeMetadata>> {
        let url = format!("{}/tv/{}/season/{}", self.base_url, tmdb_id, season);
        let params = [
            ("api_key", self.api_key.as_str()),
            ("include_adult", "false"),
            ("language", "en-US"),
        ];

        let body = self
            .fetch
            .get_text(BREAKER_KEY, &url, &params)
            .await
            .map_err(|e| Error::Other(redact_secrets(&e.to_string())))?;

        let resp: TmdbSeasonResponse = serde_json::from_str(&body)
            .map_err(|e| Error::Other(format!("failed to parse TMDB season episodes: {e}")))?;

        let episodes = resp
            .episodes
            .into_iter()
            .map(|ep| EpisodeMetadata {
                season: ep.season_number,
                episode: ep.episode_number,
                title: ep.name,
                air_date: ep.air_date,
            })
            .collect();

        Ok(episodes)
    }
}
