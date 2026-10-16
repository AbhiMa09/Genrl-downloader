use serde::Deserialize;

use crate::error::Result;
use crate::http::{Gate, GetOptions, Service};
use crate::models::ListStatus;
use crate::planner::ListEntry;
use crate::settings::Settings;

use super::models::MalEntry;

pub const JIKAN_USER_STATE_KEY: &str = "mal_public_username";
pub const ACCOUNT_SOURCE_STATE_KEY: &str = "mal_account_source";

#[derive(Debug, Deserialize)]
pub(crate) struct JikanUserAnimelistResponse {
    pub(crate) data: Vec<JikanUserAnimelistEntry>,
    #[serde(default)]
    pub(crate) pagination: Option<JikanPagination>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanPagination {
    #[serde(default)]
    pub(crate) has_next_page: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanUserAnimelistEntry {
    pub(crate) entry: JikanEntryNode,
    #[serde(default)]
    pub(crate) score: Option<i64>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) episodes_seen: Option<i64>,
    #[serde(default)]
    pub(crate) updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanEntryNode {
    pub(crate) mal_id: i64,
    pub(crate) title: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) images: Option<JikanImages>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct JikanImages {
    #[serde(default)]
    pub(crate) jpg: Option<JikanImageJpg>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct JikanImageJpg {
    #[serde(default)]
    pub(crate) image_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanAnimeDetailResponse {
    pub(crate) data: JikanAnimeDetailData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanAnimeDetailData {
    #[allow(dead_code)]
    pub(crate) mal_id: i64,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) title_english: Option<String>,
    #[serde(default)]
    pub(crate) title_japanese: Option<String>,
    #[serde(default)]
    pub(crate) titles: Option<Vec<JikanTitleItem>>,
    #[serde(rename = "type", default)]
    pub(crate) media_type: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) episodes: Option<i64>,
    #[serde(default)]
    pub(crate) year: Option<i64>,
    #[serde(default)]
    pub(crate) relations: Option<Vec<JikanRelationItem>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanTitleItem {
    #[serde(rename = "type", default)]
    #[allow(dead_code)]
    pub(crate) title_type: String,
    pub(crate) title: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanRelationItem {
    pub(crate) relation: String,
    #[serde(default)]
    pub(crate) entry: Vec<JikanRelationSubEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JikanRelationSubEntry {
    pub(crate) mal_id: i64,
    #[serde(rename = "type", default)]
    pub(crate) entry_type: String,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MalLoadJsonItem {
    pub(crate) status: i32,
    #[serde(default)]
    pub(crate) score: Option<i64>,
    #[serde(default)]
    pub(crate) num_watched_episodes: Option<i64>,
    #[serde(default)]
    pub(crate) updated_at: Option<serde_json::Value>,
    pub(crate) anime_title: serde_json::Value,
    #[serde(default)]
    pub(crate) anime_title_eng: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) anime_num_episodes: Option<i64>,
    #[serde(default)]
    pub(crate) anime_airing_status: Option<i32>,
    pub(crate) anime_id: i64,
    #[serde(default)]
    pub(crate) anime_media_type_string: Option<String>,
    #[serde(default)]
    pub(crate) anime_image_path: Option<String>,
    #[serde(default)]
    pub(crate) anime_start_date_string: Option<String>,
}

pub(crate) fn value_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

pub(crate) fn parse_start_year(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split(['-', '/', '.']).collect();
    for p in &parts {
        if p.len() == 4 {
            if let Ok(y) = p.parse::<i64>() {
                if (1950..=2050).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    let last = parts.last()?;
    if last.len() == 2 {
        if let Ok(yy) = last.parse::<i64>() {
            let y = if yy <= 50 { 2000 + yy } else { 1900 + yy };
            return Some(y);
        }
    }
    None
}

/// Fetch a user's anime list directly from MAL's public load.json endpoint.
pub async fn fetch_mal_public_load_json(gate: &Gate, username: &str) -> Result<Vec<MalEntry>> {
    let mut out = Vec::new();
    let mut offset = 0usize;
    loop {
        let url = format!(
            "https://myanimelist.net/animelist/{username}/load.json?offset={offset}&status=7"
        );
        let fetched = gate
            .get_with(Service::Mal, &url, &GetOptions::default())
            .await?;
        let items: Vec<MalLoadJsonItem> = fetched.json()?;
        if items.is_empty() {
            break;
        }
        let count = items.len();
        for item in items {
            let list_status = match item.status {
                1 => ListStatus::Watching,
                2 => ListStatus::Completed,
                3 => ListStatus::OnHold,
                4 => ListStatus::Dropped,
                6 => ListStatus::PlanToWatch,
                _ => ListStatus::PlanToWatch,
            };
            let title = value_to_string(&item.anime_title)
                .unwrap_or_else(|| format!("MAL #{}", item.anime_id));
            let mut names = vec![title.clone()];
            let eng = item.anime_title_eng.as_ref().and_then(value_to_string);
            if let Some(ref e) = eng {
                if !names.contains(e) {
                    names.push(e.clone());
                }
            }
            let airing = match item.anime_airing_status {
                Some(1) => Some("currently_airing".to_string()),
                Some(2) => Some("finished_airing".to_string()),
                Some(3) => Some("not_yet_aired".to_string()),
                _ => None,
            };
            let start_year = item
                .anime_start_date_string
                .as_deref()
                .and_then(parse_start_year);
            let updated_at = item.updated_at.as_ref().and_then(value_to_string);

            let entry = ListEntry {
                mal_id: item.anime_id,
                title: title.clone(),
                english_title: eng,
                list_status,
                watched_count: item.num_watched_episodes.unwrap_or(0).max(0),
                total_episodes: item.anime_num_episodes.filter(|n| *n > 0),
                start_year,
                auto: matches!(list_status, ListStatus::Watching),
                episode_offset: 0,
                names,
                franchise: item.anime_id,
                season: 1,
            };

            out.push(MalEntry {
                entry,
                score: item.score,
                updated_at,
                media_type: item.anime_media_type_string.clone(),
                airing_status: airing,
                source: "jikan".to_string(),
                poster_url: item.anime_image_path.clone(),
                synopsis: None,
            });
        }
        offset += count;
        if count < 300 || offset >= 10000 {
            break;
        }
    }
    Ok(out)
}

/// Fetch a user's anime list from Jikan (`/v4/users/{username}/animelist`) through Gate.
pub async fn fetch_jikan_list(
    gate: &Gate,
    settings: &Settings,
    username: &str,
) -> Result<Vec<MalEntry>> {
    let mut out = Vec::new();
    let raw_base = settings.jikan_base_url.trim_end_matches('/');
    let base = if raw_base.ends_with("/v4") {
        raw_base.to_string()
    } else {
        format!("{raw_base}/v4")
    };
    let mut page = 1;

    loop {
        let url = format!("{base}/users/{username}/animelist?page={page}");
        let fetched = gate
            .get_with(Service::Jikan, &url, &GetOptions::default())
            .await?;
        let res: JikanUserAnimelistResponse = fetched.json()?;

        for item in &res.data {
            let status_str = item.status.as_deref().unwrap_or("plan_to_watch");
            let list_status = ListStatus::parse(status_str);
            let title = item.entry.title.clone();
            let mal_id = item.entry.mal_id;

            let entry = ListEntry {
                mal_id,
                title: title.clone(),
                english_title: None,
                list_status,
                watched_count: item.episodes_seen.unwrap_or(0).max(0),
                total_episodes: None,
                start_year: None,
                auto: matches!(list_status, ListStatus::Watching),
                episode_offset: 0,
                names: vec![title],
                franchise: mal_id,
                season: 1,
            };

            out.push(MalEntry {
                entry,
                score: item.score,
                updated_at: item.updated_at.clone(),
                media_type: None,
                airing_status: None,
                source: "jikan".to_string(),
                poster_url: None,
                synopsis: None,
            });
        }

        let has_next = res
            .pagination
            .as_ref()
            .map(|p| p.has_next_page)
            .unwrap_or(false);
        if !has_next || res.data.is_empty() || page >= 20 {
            break;
        }
        page += 1;
    }

    Ok(out)
}

/// Parse Jikan animelist from JSON body text.
pub fn parse_jikan_animelist(text: &str) -> Result<Vec<MalEntry>> {
    let res: JikanUserAnimelistResponse = serde_json::from_str(text)?;
    let mut out = Vec::new();
    for item in &res.data {
        let status_str = item.status.as_deref().unwrap_or("plan_to_watch");
        let list_status = ListStatus::parse(status_str);
        let title = item.entry.title.clone();
        let mal_id = item.entry.mal_id;

        let entry = ListEntry {
            mal_id,
            title: title.clone(),
            english_title: None,
            list_status,
            watched_count: item.episodes_seen.unwrap_or(0).max(0),
            total_episodes: None,
            start_year: None,
            auto: matches!(list_status, ListStatus::Watching),
            episode_offset: 0,
            names: vec![title],
            franchise: mal_id,
            season: 1,
        };

        out.push(MalEntry {
            entry,
            score: item.score,
            updated_at: item.updated_at.clone(),
            media_type: None,
            airing_status: None,
            source: "jikan".to_string(),
            poster_url: None,
            synopsis: None,
        });
    }
    Ok(out)
}
