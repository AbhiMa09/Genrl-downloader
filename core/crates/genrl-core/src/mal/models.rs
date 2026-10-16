use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::models::ListStatus;
use crate::planner::ListEntry;

#[derive(Debug, Deserialize)]
pub(crate) struct RawListStatus {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) score: Option<i64>,
    #[serde(default)]
    pub(crate) num_episodes_watched: Option<i64>,
    #[serde(default)]
    pub(crate) updated_at: Option<String>,
}

/// One anime as MAL reports it (list node or relation node).
#[derive(Debug, Clone, Deserialize)]
pub struct Node {
    pub(crate) id: i64,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) main_title: Option<String>,
    #[serde(default)]
    pub(crate) alternative_titles: Option<AltTitles>,
    /// `Some(0)` means "unknown"; it maps to `None`.
    #[serde(default)]
    pub(crate) num_episodes: Option<i64>,
    #[serde(default)]
    pub(crate) start_season: Option<SeasonInfo>,
    #[serde(default)]
    pub(crate) media_type: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) main_picture: Option<MainPicture>,
    #[serde(default)]
    pub(crate) synopsis: Option<String>,
}

impl Node {
    pub fn id(&self) -> i64 {
        self.id
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MainPicture {
    #[serde(default)]
    pub(crate) medium: Option<String>,
    #[serde(default)]
    pub(crate) large: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct AltTitles {
    #[serde(default)]
    pub(crate) synonyms: Vec<String>,
    #[serde(default)]
    pub(crate) en: Option<String>,
    #[serde(default)]
    pub(crate) ja: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SeasonInfo {
    #[serde(default)]
    pub(crate) year: Option<i64>,
}

/// `GET /anime/{id}?fields=related_anime`.
#[derive(Debug, Deserialize)]
pub(crate) struct AnimeDetail {
    #[serde(default)]
    pub(crate) related_anime: Vec<Relation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Relation {
    pub node: Node,
    pub relation_type: String,
}

/// A list row once it has been parsed into planner shape, plus the raw fields
/// the diff stores in `mal_entries` and `shows`.
#[derive(Debug, Clone, PartialEq)]
pub struct MalEntry {
    pub entry: ListEntry,
    pub score: Option<i64>,
    pub updated_at: Option<String>,
    pub media_type: Option<String>,
    pub airing_status: Option<String>,
    pub source: String,
    pub poster_url: Option<String>,
    pub synopsis: Option<String>,
}

impl MalEntry {
    pub fn mal_id(&self) -> i64 {
        self.entry.mal_id
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FetchedList {
    pub entries: Vec<MalEntry>,
    pub relations_failed: usize,
}

#[derive(Debug, Deserialize)]
pub struct Account {
    pub id: i64,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ListItem {
    pub(crate) node: Node,
    pub(crate) list_status: RawListStatus,
}

pub(crate) fn to_mal_entry(item: &ListItem) -> MalEntry {
    let node = &item.node;
    let alt = node.alternative_titles.clone().unwrap_or_default();
    let title = node
        .title
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| node.main_title.clone().filter(|t| !t.is_empty()))
        .unwrap_or_else(|| format!("MAL #{}", node.id));

    // The main title first, then English, Japanese and synonyms, deduplicated
    // case-insensitively: this is the matching engine's whole input set.
    let mut names: Vec<String> = Vec::new();
    let candidates = [
        Some(title.clone()),
        node.main_title.clone(),
        alt.en.clone(),
        alt.ja.clone(),
    ]
    .into_iter()
    .flatten()
    .chain(alt.synonyms.iter().cloned());
    for name in candidates {
        let name = name.trim().to_string();
        if !name.is_empty() && !names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            names.push(name);
        }
    }

    let status = ListStatus::parse(&item.list_status.status);
    let entry = ListEntry {
        mal_id: node.id,
        title,
        english_title: alt.en.filter(|t| !t.is_empty()),
        list_status: status,
        watched_count: item.list_status.num_episodes_watched.unwrap_or(0).max(0),
        total_episodes: node.num_episodes.filter(|n| *n > 0),
        start_year: node.start_season.as_ref().and_then(|s| s.year),
        auto: matches!(status, ListStatus::Watching),
        episode_offset: 0,
        names,
        franchise: node.id,
        season: 1,
    };

    let poster_url = node
        .main_picture
        .as_ref()
        .and_then(|p| p.large.clone().or_else(|| p.medium.clone()));

    MalEntry {
        entry,
        score: item.list_status.score,
        updated_at: item.list_status.updated_at.clone(),
        media_type: node.media_type.clone(),
        airing_status: node.status.clone(),
        source: "mal".to_string(),
        poster_url,
        synopsis: node.synopsis.clone(),
    }
}

pub(crate) fn show_type(kind: Option<&str>) -> &'static str {
    match kind {
        Some("movie") => "movie",
        Some("ova") => "ova",
        Some("ona") | Some("music") => "ona",
        Some("special") | Some("tv_special") => "special",
        _ => "tv",
    }
}

pub(crate) fn airing_status(s: Option<&str>) -> Option<&'static str> {
    match s {
        Some("currently_airing") => Some("airing"),
        Some("finished_airing") => Some("finished"),
        Some("not_yet_aired") => Some("not_yet_aired"),
        // Unknown: keep whatever the previous sync knew.
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The season chain
// ---------------------------------------------------------------------------

/// Fill `franchise`, `season` and `episode_offset` from MAL's sequel/prequel
/// relations: each connected component is one chain, ordered along the links,
/// offset by the cumulative episode counts of the earlier seasons. Entries
/// with no relations (or whose lookups failed) stay their own chain.
pub fn build_chains(entries: &mut [MalEntry], relations: &HashMap<i64, Vec<Relation>>) {
    let index: HashMap<i64, usize> = entries
        .iter()
        .enumerate()
        .map(|(i, m)| (m.mal_id(), i))
        .collect();
    let id_set: HashSet<i64> = index.keys().copied().collect();

    // `precedes` maps an id to the ids that come after it; `links` is the same
    // graph made undirected, for grouping.
    let mut precedes: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut links: HashMap<i64, HashSet<i64>> = HashMap::new();

    for (from, rels) in relations {
        if !id_set.contains(from) {
            continue;
        }
        for rel in rels {
            let to = rel.node.id;
            if !id_set.contains(&to) || to == *from {
                continue;
            }
            let (before, after) = match rel.relation_type.as_str() {
                "sequel" => (*from, to),
                "prequel" => (to, *from),
                _ => continue,
            };
            precedes.entry(before).or_default().insert(after);
            links.entry(before).or_default().insert(after);
            links.entry(after).or_default().insert(before);
        }
    }

    // Connected components (undirected), each stable-ordered for deterministic
    // franchise numbering.
    let mut seen: HashSet<i64> = HashSet::new();
    let mut components: Vec<Vec<i64>> = Vec::new();
    for &id in &index.keys().copied().collect::<Vec<i64>>() {
        if !seen.insert(id) {
            continue;
        }
        let mut stack = vec![id];
        let mut component = Vec::new();
        while let Some(current) = stack.pop() {
            component.push(current);
            if let Some(neighbours) = links.get(&current) {
                for &next in neighbours {
                    if seen.insert(next) {
                        stack.push(next);
                    }
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }

    for component in &components {
        // Kahn's order: emit a node when nothing still-remaining points at it
        // as a successor — i.e., no remaining node `precedes` it.  A cycle
        // falls back to the smallest id so the order stays stable anyway.
        let mut remaining: HashSet<i64> = component.iter().copied().collect();
        let mut ordered: Vec<i64> = Vec::with_capacity(component.len());
        while !remaining.is_empty() {
            let mut pick = None;
            for &id in &remaining {
                // id is ready when no other remaining node lists id as a
                // successor (i.e., nothing that is still unprocessed comes
                // before id in the chain).
                let has_predecessor = precedes.values().any(|succs| succs.contains(&id))
                    && precedes
                        .iter()
                        .any(|(pred, succs)| succs.contains(&id) && remaining.contains(pred));
                let ready = !has_predecessor;
                if ready {
                    pick = Some(id);
                    break;
                }
            }
            let id = match pick {
                Some(id) => id,
                None => match remaining.iter().min() {
                    Some(&first) => first,
                    None => break,
                },
            };
            remaining.remove(&id);
            ordered.push(id);
        }

        // The smallest mal id names the franchise, so it is stable across
        // syncs; seasons run along the ordered chain.
        let franchise = component.first().copied().unwrap_or_default();
        let mut offset: Option<i64> = Some(0);
        for (position, id) in ordered.iter().enumerate() {
            let total = entries.get(index[id]).and_then(|m| m.entry.total_episodes);
            if let Some(entry) = entries.get_mut(index[id]) {
                entry.entry.franchise = franchise;
                entry.entry.season = position as i64 + 1;
                entry.entry.episode_offset = offset.unwrap_or(0);
            }
            offset = offset.and_then(|o| total.map(|t| o + t));
        }
    }
}
