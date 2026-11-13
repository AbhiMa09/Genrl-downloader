use crate::error::{Error, Result};
use std::path::Path;

pub const VIDEO_EXTS: &[&str] = &["mkv", "mp4", "avi", "m4v", "mov", "ts", "wmv"];
pub const SUBTITLE_EXTS: &[&str] = &["srt", "ass", "ssa", "sub", "vtt"];
pub const MIN_VIDEO_BYTES: u64 = 50 * 1024 * 1024; // 50 MB

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFile {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarSubtitle {
    pub source_path: String,
    pub lang_suffix: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedMovie {
    pub main_video: TorrentFile,
    pub subtitles: Vec<SidecarSubtitle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MovieSelectionOutcome {
    Selected(SelectedMovie),
    AmbiguousNearEqualVideos {
        video_a: TorrentFile,
        video_b: TorrentFile,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesMappedEpisode {
    pub file: TorrentFile,
    pub season: u32,
    pub episode: f64,
    pub subtitles: Vec<SidecarSubtitle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectedSeriesPack {
    pub episodes: Vec<SeriesMappedEpisode>,
    pub unmapped_files: Vec<String>,
}

pub fn is_video_path(path_str: &str) -> bool {
    let path = Path::new(path_str);
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| VIDEO_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_subtitle_path(path_str: &str) -> bool {
    let path = Path::new(path_str);
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| SUBTITLE_EXTS.contains(&ext.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn is_sample(path_str: &str) -> bool {
    path_str.to_ascii_lowercase().contains("sample")
}

pub fn detect_lang_suffix(sub_path: &str) -> Option<String> {
    let path = Path::new(sub_path);
    let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
    if stem.ends_with(".en") || stem.ends_with(".eng") || stem.contains("english") {
        Some("en".to_string())
    } else if stem.ends_with(".ja")
        || stem.ends_with(".jp")
        || stem.ends_with(".jpn")
        || stem.contains("japanese")
    {
        Some("ja".to_string())
    } else if stem.ends_with(".es") || stem.ends_with(".spa") || stem.contains("spanish") {
        Some("es".to_string())
    } else if stem.ends_with(".fr") || stem.ends_with(".fre") || stem.contains("french") {
        Some("fr".to_string())
    } else if stem.ends_with(".de") || stem.ends_with(".ger") || stem.contains("german") {
        Some("de".to_string())
    } else if let Some(last_dot) = stem.rfind('.') {
        let suffix = &stem[last_dot + 1..];
        if suffix.len() == 2 || suffix.len() == 3 {
            Some(suffix.to_string())
        } else {
            None
        }
    } else {
        None
    }
}

pub fn find_matching_subtitles(
    video_path: &str,
    all_files: &[TorrentFile],
) -> Vec<SidecarSubtitle> {
    let v_path = Path::new(video_path);
    let v_stem = v_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let mut subs = Vec::new();

    for file in all_files {
        if !is_subtitle_path(&file.path) {
            continue;
        }
        let s_path = Path::new(&file.path);
        let s_stem = s_path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let in_subs_dir = s_path
            .parent()
            .map(|p| {
                let p_str = p.to_string_lossy().to_ascii_lowercase();
                p_str.contains("sub")
            })
            .unwrap_or(false);

        if s_stem.starts_with(v_stem) || in_subs_dir {
            subs.push(SidecarSubtitle {
                source_path: file.path.clone(),
                lang_suffix: detect_lang_suffix(&file.path),
            });
        }
    }
    subs
}

/// Select main movie video and sidecar subtitles from a torrent file list.
pub fn select_movie_files(files: &[TorrentFile]) -> Result<MovieSelectionOutcome> {
    let mut eligible_videos: Vec<&TorrentFile> = files
        .iter()
        .filter(|f| {
            is_video_path(&f.path) && !is_sample(&f.path) && f.size_bytes >= MIN_VIDEO_BYTES
        })
        .collect();

    if eligible_videos.is_empty() {
        return Err(Error::Other(
            "no valid video file found in torrent (files under 50 MB, samples, or non-video)"
                .into(),
        ));
    }

    eligible_videos.sort_by_key(|a| std::cmp::Reverse(a.size_bytes));

    if eligible_videos.len() >= 2 {
        let top1 = eligible_videos[0];
        let top2 = eligible_videos[1];
        let diff = top1.size_bytes.saturating_sub(top2.size_bytes);
        if diff <= top1.size_bytes / 10 {
            return Ok(MovieSelectionOutcome::AmbiguousNearEqualVideos {
                video_a: (*top1).clone(),
                video_b: (*top2).clone(),
            });
        }
    }

    let main_video = (*eligible_videos[0]).clone();
    let subtitles = find_matching_subtitles(&main_video.path, files);

    Ok(MovieSelectionOutcome::Selected(SelectedMovie {
        main_video,
        subtitles,
    }))
}

/// Select and map series episodes and sidecar subtitles from a season pack or torrent file list.
pub fn select_series_pack_files(files: &[TorrentFile]) -> Result<SelectedSeriesPack> {
    let eligible_videos: Vec<&TorrentFile> = files
        .iter()
        .filter(|f| {
            is_video_path(&f.path) && !is_sample(&f.path) && f.size_bytes >= MIN_VIDEO_BYTES
        })
        .collect();

    if eligible_videos.is_empty() {
        return Err(Error::Other(
            "no valid video files found in series pack".into(),
        ));
    }

    let mut episodes = Vec::new();
    let mut unmapped_files = Vec::new();

    for file in eligible_videos {
        let p = Path::new(&file.path);
        let filename = p.file_name().and_then(|f| f.to_str()).unwrap_or(&file.path);
        let parsed = crate::matching::release_parse::parse_media_release(filename);

        if let Some(ep) = parsed.episode {
            let season = parsed.season.unwrap_or(1);
            let subtitles = find_matching_subtitles(&file.path, files);
            episodes.push(SeriesMappedEpisode {
                file: (*file).clone(),
                season,
                episode: ep,
                subtitles,
            });
        } else {
            unmapped_files.push(file.path.clone());
        }
    }

    if episodes.is_empty() {
        return Err(Error::Other(
            "no episodes could be mapped in series pack (unparseable file names)".into(),
        ));
    }

    episodes.sort_by(|a, b| {
        a.season.cmp(&b.season).then_with(|| {
            a.episode
                .partial_cmp(&b.episode)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    Ok(SelectedSeriesPack {
        episodes,
        unmapped_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_movie_with_sample_and_nfo() {
        let files = vec![
            TorrentFile {
                path: "Dune.2024.1080p.mkv".into(),
                size_bytes: 8_000_000_000,
            },
            TorrentFile {
                path: "Sample/sample.mkv".into(),
                size_bytes: 60_000_000,
            },
            TorrentFile {
                path: "movie.nfo".into(),
                size_bytes: 2_000,
            },
            TorrentFile {
                path: "Dune.2024.1080p.en.srt".into(),
                size_bytes: 50_000,
            },
            TorrentFile {
                path: "malware.exe".into(),
                size_bytes: 1_000_000,
            },
        ];

        let outcome = select_movie_files(&files).unwrap();
        match outcome {
            MovieSelectionOutcome::Selected(movie) => {
                assert_eq!(movie.main_video.path, "Dune.2024.1080p.mkv");
                assert_eq!(movie.subtitles.len(), 1);
                assert_eq!(movie.subtitles[0].source_path, "Dune.2024.1080p.en.srt");
                assert_eq!(movie.subtitles[0].lang_suffix.as_deref(), Some("en"));
            }
            _ => panic!("expected Selected"),
        }
    }

    #[test]
    fn test_near_equal_videos_raise_ambiguity() {
        let files = vec![
            TorrentFile {
                path: "Movie.Theatrical.1080p.mkv".into(),
                size_bytes: 10_000_000_000,
            },
            TorrentFile {
                path: "Movie.DirectorsCut.1080p.mkv".into(),
                size_bytes: 9_500_000_000,
            },
        ];

        let outcome = select_movie_files(&files).unwrap();
        match outcome {
            MovieSelectionOutcome::AmbiguousNearEqualVideos { video_a, video_b } => {
                assert_eq!(video_a.path, "Movie.Theatrical.1080p.mkv");
                assert_eq!(video_b.path, "Movie.DirectorsCut.1080p.mkv");
            }
            _ => panic!("expected AmbiguousNearEqualVideos"),
        }
    }

    #[test]
    fn test_no_valid_video_file_errors() {
        let files = vec![
            TorrentFile {
                path: "sample.mkv".into(),
                size_bytes: 20_000_000,
            },
            TorrentFile {
                path: "readme.txt".into(),
                size_bytes: 1_000,
            },
        ];
        assert!(select_movie_files(&files).is_err());
    }

    #[test]
    fn test_series_pack_mapping_and_unmapped_logging() {
        let files = vec![
            TorrentFile {
                path: "Show.S01E01.1080p.mkv".into(),
                size_bytes: 1_200_000_000,
            },
            TorrentFile {
                path: "Show.S01E02.1080p.mkv".into(),
                size_bytes: 1_200_000_000,
            },
            TorrentFile {
                path: "Extras.Featurette.1080p.mkv".into(),
                size_bytes: 800_000_000,
            },
        ];

        let pack = select_series_pack_files(&files).unwrap();
        assert_eq!(pack.episodes.len(), 2);
        assert_eq!(pack.episodes[0].episode, 1.0);
        assert_eq!(pack.episodes[1].episode, 2.0);
        assert_eq!(
            pack.unmapped_files,
            vec!["Extras.Featurette.1080p.mkv".to_string()]
        );
    }
}
