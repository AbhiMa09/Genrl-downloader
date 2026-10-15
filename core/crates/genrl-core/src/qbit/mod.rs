//! The qBittorrent WebUI client.
//!
//! Everything the app does with qBittorrent goes through here, and every call
//! itself goes through the shared [`Gate`] (`Service::Qbit`), so the limiter,
//! the circuit breaker and the backoff apply to it like any other service.
//!
//! Three design requirements shape this module:
//!
//! * **session** — log in once, keep the session cookie on every call
//!   (`QBT_SID_…` in 5.x, `SID` in 4.x), and re-log in automatically when the
//!   API answers 403 (expired session);
//! * **version** — 4.x spelled the controls `pause`/`resume`, 5.x renamed them
//!   `stop`/`start`; we read the version once and hide the difference;
//! * **health** — a launch health check that can start `qbittorrent-nox` when
//!   the settings allow it, and otherwise explains how to fix it.

pub mod client;
pub mod service;
pub mod types;

pub use client::*;
pub use service::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_generation_from_version_strings() {
        assert_eq!(ApiGen::parse("v5.2.4"), ApiGen::V5);
        assert_eq!(ApiGen::parse("v4.6.7"), ApiGen::V4);
        assert_eq!(ApiGen::parse("5.0.0"), ApiGen::V5);
        assert_eq!(ApiGen::parse(""), ApiGen::V4, "unknown falls back");
    }

    #[test]
    fn control_paths_follow_the_version() {
        assert_eq!(ApiGen::V4.control_path(true), "resume");
        assert_eq!(ApiGen::V4.control_path(false), "pause");
        assert_eq!(ApiGen::V5.control_path(true), "start");
        assert_eq!(ApiGen::V5.control_path(false), "stop");
    }

    #[test]
    fn magnets_yield_their_info_hash() {
        let hash = "ef3e7ad1b12bdd9fc341691d8866cd1fa8374a4b";
        let magnet = format!("magnet:?xt=urn:btih:{hash}&dn=Show&tr=udp://tracker.example");
        assert_eq!(magnet_info_hash(&magnet).as_deref(), Some(hash));
        assert_eq!(
            magnet_info_hash("magnet:?dn=NoHash").as_deref(),
            None,
            "no xt, no hash"
        );
        assert_eq!(
            magnet_info_hash("magnet:?xt=urn:btih:SHORT").as_deref(),
            None,
            "must be a full sha1"
        );
        assert_eq!(magnet_info_hash("https://example.com"), None);
    }

    #[test]
    fn torrent_states_are_classified() {
        let stopped = Torrent {
            hash: "h".into(),
            name: "n".into(),
            progress: 0.5,
            dlspeed: 0,
            upspeed: 0,
            eta: 0,
            num_seeds: 0,
            num_leechs: 0,
            size: 1,
            state: "stoppedDL".into(),
            save_path: String::new(),
            content_path: String::new(),
            amount_left: 1,
            added_on: 0,
            completion_on: 0,
            category: String::new(),
        };
        assert!(stopped.is_stopped());
        assert!(!stopped.is_complete());
        assert!(!stopped.is_in_trouble());

        let done = Torrent {
            state: "uploading".into(),
            progress: 1.0,
            ..stopped.clone()
        };
        assert!(done.is_complete());
        assert!(!done.is_stopped());

        let broken = Torrent {
            state: "error".into(),
            progress: 0.1,
            ..stopped
        };
        assert!(broken.is_in_trouble());
    }
}
