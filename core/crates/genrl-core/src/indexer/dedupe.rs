use std::collections::HashMap;

use super::types::Release;

/// Deduplicates releases across multiple indexers.
///
/// Priority:
/// 1. Dedupes by info_hash (lowercased) if present.
/// 2. If info_hash is absent, dedupes by lowercase normalized title and size_bytes.
///
/// In either case, preserves the candidate with the higher seed count.
pub fn dedupe_releases(releases: Vec<Release>) -> Vec<Release> {
    let mut by_hash: HashMap<String, Release> = HashMap::new();
    let mut no_hash: HashMap<String, Release> = HashMap::new();

    for release in releases {
        if let Some(ref hash) = release.info_hash {
            let key = hash.trim().to_ascii_lowercase();
            if !key.is_empty() {
                match by_hash.get_mut(&key) {
                    Some(existing) => {
                        if release.seeders > existing.seeders {
                            *existing = release;
                        }
                    }
                    None => {
                        by_hash.insert(key, release);
                    }
                }
                continue;
            }
        }

        let normalized_title = normalize_title_for_dedupe(&release.title);
        let key = format!("{}:{}", normalized_title, release.size_bytes);
        match no_hash.get_mut(&key) {
            Some(existing) => {
                if release.seeders > existing.seeders {
                    *existing = release;
                }
            }
            None => {
                no_hash.insert(key, release);
            }
        }
    }

    let mut result: Vec<Release> = by_hash.into_values().collect();
    result.extend(no_hash.into_values());
    result.sort_by_key(|r| std::cmp::Reverse(r.seeders));
    result
}

fn normalize_title_for_dedupe(title: &str) -> String {
    title
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::types::DownloadSource;

    fn sample_release(
        title: &str,
        size: u64,
        seeders: u32,
        hash: Option<&str>,
        indexer: &str,
    ) -> Release {
        Release {
            title: title.to_string(),
            indexer_id: indexer.to_string(),
            size_bytes: size,
            seeders,
            leechers: 1,
            download: DownloadSource::Magnet("magnet:?xt=test".into()),
            info_hash: hash.map(str::to_string),
            categories: vec![2000],
            published_date: None,
            trusted: false,
            remake: false,
        }
    }

    #[test]
    fn dedupe_by_info_hash_keeps_highest_seeders() {
        let r1 = sample_release(
            "Movie.2021.1080p",
            2_000_000_000,
            10,
            Some("AABBCC11"),
            "idx1",
        );
        let r2 = sample_release(
            "Different.Title.Same.Hash",
            2_000_000_000,
            45,
            Some("aabbcc11"),
            "idx2",
        );
        let r3 = sample_release(
            "Movie.2021.1080p",
            2_000_000_000,
            5,
            Some("AABBCC11"),
            "idx3",
        );

        let deduped = dedupe_releases(vec![r1, r2, r3]);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].seeders, 45);
        assert_eq!(deduped[0].indexer_id, "idx2");
    }

    #[test]
    fn dedupe_by_title_and_size_keeps_highest_seeders() {
        let r1 = sample_release(
            "[Group] Movie (2021) 1080p",
            1_500_000_000,
            12,
            None,
            "idx1",
        );
        let r2 = sample_release("Group Movie 2021 1080p", 1_500_000_000, 80, None, "idx2");
        let r3 = sample_release("Other Movie 2021 1080p", 1_500_000_000, 30, None, "idx3");

        let deduped = dedupe_releases(vec![r1, r2, r3]);
        assert_eq!(deduped.len(), 2);
        let found = deduped
            .iter()
            .find(|r| r.indexer_id == "idx2")
            .expect("preserved idx2");
        assert_eq!(found.seeders, 80);
    }
}
