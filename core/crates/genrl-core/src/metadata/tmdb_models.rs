use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbMovieItem {
    pub id: u32,
    pub title: String,
    pub original_title: Option<String>,
    pub release_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbMovieSearchResponse {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub results: Vec<TmdbMovieItem>,
    #[serde(default)]
    pub total_results: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbTvItem {
    pub id: u32,
    pub name: String,
    pub original_name: Option<String>,
    pub first_air_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbTvSearchResponse {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub results: Vec<TmdbTvItem>,
    #[serde(default)]
    pub total_results: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbExternalIdsResponse {
    pub id: u32,
    pub imdb_id: Option<String>,
    pub tvdb_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbEpisodeItem {
    pub id: u32,
    pub episode_number: u32,
    pub season_number: u32,
    pub name: String,
    pub air_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbSeasonResponse {
    #[serde(default)]
    pub id: u32,
    pub season_number: u32,
    pub name: Option<String>,
    #[serde(default)]
    pub episodes: Vec<TmdbEpisodeItem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbSeasonSummary {
    pub id: u32,
    pub season_number: u32,
    pub name: Option<String>,
    pub episode_count: Option<u32>,
    pub air_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct TmdbTvDetailsResponse {
    pub id: u32,
    pub name: String,
    pub status: Option<String>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    #[serde(default)]
    pub seasons: Vec<TmdbSeasonSummary>,
}
