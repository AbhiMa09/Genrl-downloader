use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

pub const MOVIES_SUBFOLDER: &str = "Movies";
pub const SERIES_SUBFOLDER: &str = "Series";
pub const MAX_COMPONENT_LEN: usize = 120;

/// Sanitize a path component for filesystem safety across Linux and Windows.
pub fn sanitize_component(raw: &str) -> Result<String> {
    let mut cleaned = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c.is_control() {
            continue;
        }
        match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => {
                cleaned.push(' ');
            }
            other => cleaned.push(other),
        }
    }

    // Collapse multiple spaces
    let collapsed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim_matches(|c: char| c == ' ' || c == '.');

    if trimmed.is_empty() {
        return Err(Error::Other(
            "path component is empty after sanitization".into(),
        ));
    }
    if trimmed == "." || trimmed == ".." {
        return Err(Error::Other(
            "directory traversal detected in path component".into(),
        ));
    }

    // Limit length to 120 characters preserving char boundaries
    let truncated: String = trimmed.chars().take(MAX_COMPONENT_LEN).collect();
    let final_clean = truncated.trim_end_matches([' ', '.']);

    if final_clean.is_empty() {
        return Err(Error::Other(
            "path component is empty after length trimming".into(),
        ));
    }

    Ok(final_clean.to_string())
}

/// Generate destination path for a movie.
pub fn movie_path(
    library_root: &Path,
    title: &str,
    year: Option<i64>,
    ext: &str,
) -> Result<PathBuf> {
    let clean_title = sanitize_component(title)?;
    let clean_ext = ext.trim_start_matches('.').trim();
    if clean_ext.is_empty() {
        return Err(Error::Other("file extension is empty".into()));
    }

    let folder_and_file = match year {
        Some(y) => sanitize_component(&format!("{clean_title} ({y})"))?,
        None => clean_title,
    };

    let filename = format!("{folder_and_file}.{clean_ext}");
    Ok(library_root
        .join(MOVIES_SUBFOLDER)
        .join(&folder_and_file)
        .join(filename))
}

/// Generate destination path for a series episode.
pub fn series_path(
    library_root: &Path,
    title: &str,
    year: Option<i64>,
    season: i64,
    episode: i64,
    ext: &str,
) -> Result<PathBuf> {
    let clean_title = sanitize_component(title)?;
    let clean_ext = ext.trim_start_matches('.').trim();
    if clean_ext.is_empty() {
        return Err(Error::Other("file extension is empty".into()));
    }

    let show_folder = match year {
        Some(y) => sanitize_component(&format!("{clean_title} ({y})"))?,
        None => clean_title.clone(),
    };

    let s = season.max(1);
    let season_folder = format!("Season {:02}", s);

    let ep_tag = if episode > 99 {
        format!("S{:02}E{:03}", s, episode)
    } else {
        format!("S{:02}E{:02}", s, episode.max(1))
    };

    let filename = format!("{clean_title} - {ep_tag}.{clean_ext}");
    Ok(library_root
        .join(SERIES_SUBFOLDER)
        .join(&show_folder)
        .join(&season_folder)
        .join(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_component_basics() {
        assert_eq!(
            sanitize_component("Alien: Romulus").unwrap(),
            "Alien Romulus"
        );
        assert_eq!(sanitize_component("AC/DC").unwrap(), "AC DC");
        assert_eq!(sanitize_component("Film...").unwrap(), "Film");
        assert_eq!(
            sanitize_component("Hello * ? < > | \" World").unwrap(),
            "Hello World"
        );
    }

    #[test]
    fn test_sanitize_unicode_japanese_and_korean() {
        assert_eq!(sanitize_component("君の名は。").unwrap(), "君の名は。");
        assert_eq!(sanitize_component("기생충").unwrap(), "기생충");
    }

    #[test]
    fn test_sanitize_traversal_and_empty() {
        assert!(sanitize_component("..").is_err());
        assert!(sanitize_component(".").is_err());
        assert!(sanitize_component("../../etc").unwrap() == "etc"); // slashes replaced with space, words collapsed
        assert!(sanitize_component("   ...   ").is_err());
    }

    #[test]
    fn test_sanitize_long_name_cap() {
        let long = "A".repeat(200);
        let sanitized = sanitize_component(&long).unwrap();
        assert_eq!(sanitized.chars().count(), MAX_COMPONENT_LEN);
    }

    #[test]
    fn test_movie_path_layout() {
        let root = Path::new("/library");
        let p1 = movie_path(root, "Dune: Part Two", Some(2024), "mkv").unwrap();
        assert_eq!(
            p1,
            PathBuf::from("/library/Movies/Dune Part Two (2024)/Dune Part Two (2024).mkv")
        );

        let p2 = movie_path(root, "Fight Club", None, ".mp4").unwrap();
        assert_eq!(
            p2,
            PathBuf::from("/library/Movies/Fight Club/Fight Club.mp4")
        );
    }

    #[test]
    fn test_series_path_layout() {
        let root = Path::new("/library");
        let p1 = series_path(root, "Breaking Bad", Some(2008), 1, 3, "mkv").unwrap();
        assert_eq!(
            p1,
            PathBuf::from(
                "/library/Series/Breaking Bad (2008)/Season 01/Breaking Bad - S01E03.mkv"
            )
        );

        let p2 = series_path(root, "One Piece", None, 1, 1054, "mkv").unwrap();
        assert_eq!(
            p2,
            PathBuf::from("/library/Series/One Piece/Season 01/One Piece - S01E1054.mkv")
        );
    }

    #[test]
    fn test_anime_resolution_never_routes_into_movies_or_series() {
        let temp = tempfile::tempdir().unwrap();
        let db = crate::db::Db::open_in_memory().unwrap();
        db.with(|c| {
            c.execute(
                "INSERT INTO shows (mal_id, title, start_year) VALUES (1, 'Frieren', 2023)",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        let anime_folder =
            crate::library::resolve_show_folder(&db, temp.path(), 1, "Frieren", Some(2023))
                .unwrap();

        assert_eq!(anime_folder, temp.path().join("Frieren (2023)"));
        assert!(!anime_folder.starts_with(temp.path().join(MOVIES_SUBFOLDER)));
        assert!(!anime_folder.starts_with(temp.path().join(SERIES_SUBFOLDER)));
    }
}
