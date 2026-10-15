use serde::{Deserialize, Serialize};

/// Which generation of the WebUI API we are talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiGen {
    V4,
    V5,
}

impl ApiGen {
    /// `"v5.2.4"` → V5, `"v4.6.4"` → V4, anything unparseable is V4 (the
    /// pause/resume names are the older, longer-lived spelling).
    pub fn parse(version: &str) -> Self {
        let major = version
            .trim()
            .trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|first| first.parse::<u32>().ok())
            .unwrap_or(4);
        if major >= 5 {
            ApiGen::V5
        } else {
            ApiGen::V4
        }
    }

    /// The control path for starting (`true`) or stopping (`false`) torrents.
    pub fn control_path(self, want_running: bool) -> &'static str {
        match (self, want_running) {
            (ApiGen::V4, true) => "resume",
            (ApiGen::V4, false) => "pause",
            (ApiGen::V5, true) => "start",
            (ApiGen::V5, false) => "stop",
        }
    }
}

/// What the health check reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Health {
    /// Version string as qBittorrent reports it, e.g. `v5.2.4`.
    pub version: String,
    pub gen: ApiGen,
}

/// One row of `/api/v2/torrents/info`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Torrent {
    pub hash: String,
    pub name: String,
    /// `0.0 ..= 1.0`.
    pub progress: f64,
    /// Bytes per second, down and up.
    pub dlspeed: i64,
    pub upspeed: i64,
    /// Seconds; qBittorrent reports `8640000` for "unknown".
    pub eta: i64,
    #[serde(default)]
    pub num_seeds: i64,
    #[serde(default)]
    pub num_leechs: i64,
    /// Total size in bytes.
    pub size: i64,
    /// `downloading`, `metaDL`, `stalledDL`, `stoppedDL` (5.x) or
    /// `pausedDL` (4.x), `uploading`, `error`, …
    pub state: String,
    #[serde(default)]
    pub save_path: String,
    #[serde(default)]
    pub content_path: String,
    #[serde(default)]
    pub amount_left: i64,
    #[serde(default)]
    pub added_on: i64,
    #[serde(default)]
    pub completion_on: i64,
    #[serde(default)]
    pub category: String,
}

impl Torrent {
    /// Finished downloading (every byte is on disk).
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }

    /// Blocked or errored rather than merely waiting for peers.
    pub fn is_in_trouble(&self) -> bool {
        self.state.contains("error") || self.state.contains("missingFiles")
    }

    /// Queued, stopped or paused: nothing is moving right now.
    pub fn is_stopped(&self) -> bool {
        self.state.contains("stopped") || self.state.contains("paused") || self.state.contains("queued")
    }
}

/// Total transfer speeds for the status strip.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferSpeeds {
    #[serde(rename = "dl_info_speed")]
    pub download: i64,
    #[serde(rename = "up_info_speed")]
    pub upload: i64,
}

/// Extract the 40-hex info hash from a magnet link, if it carries one.
pub fn magnet_info_hash(link: &str) -> Option<String> {
    let rest = link.strip_prefix("magnet:?").or_else(|| link.strip_prefix("magnet:"))?;
    for pair in rest.trim_start_matches('?').split('&') {
        if let Some(value) = pair.strip_prefix("xt=urn:btih:") {
            let hash = value.to_lowercase();
            if hash.len() == 40 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(hash);
            }
        }
    }
    None
}
