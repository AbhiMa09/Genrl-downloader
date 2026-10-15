//! Release-name parsing and title normalization.
//!
//! Fansubs write titles every way they like: `[SubsPlease] Sousou no Frieren -
//! 28 (1080p) [ABCD1234].mkv`, `Kimi no Na wa (2016) [BD1080p]`, `[Erai-raws]
//! Frieren 2nd Season [1080p][MultiSub] (unofficial batch)`. The parser pulls
//! out group, title, season, episode, resolution, codec, source and batch-ness,
//! and strips group tags, CRC hashes and resolutions before anything is scored.

pub mod normalize;
pub mod release;

pub use normalize::*;
pub use release::*;

/// Parse a release name or filename.
pub fn parse_release(input: &str) -> Release {
    let raw = input.trim().to_string();
    let mut release = Release {
        raw: raw.clone(),
        ..Default::default()
    };

    // 1. Extension: `... .mkv` (kept for the allowlist check at finalize).
    let mut name = raw.as_str();
    if let Some((stem, ext)) = name.rsplit_once('.') {
        let ext = ext.to_ascii_lowercase();
        if VIDEO_EXTENSIONS.contains(&ext.as_str()) && !ext.contains(' ') {
            release.extension = Some(ext);
            name = stem;
        }
    }

    // 2. Leading `[Group]`.
    let group_re = re(r"^\s*\[([^\]]+)\]");
    let mut title_start = 0;
    if let Some(caps) = group_re.captures(name) {
        let candidate = caps[1].trim().to_string();
        if !is_metadata_bracket(&candidate) {
            release.group = Some(candidate);
            title_start = caps.get(0).unwrap().end();
        }
    }

    // 3. Season and episode (returns where the episode token started).
    release.marker_start = extract_season_episode(name, &mut release);

    // 4. Everything else comes from the full name: resolution, codec, source.
    extract_metadata(name, &mut release);

    // 5. Title = text before the episode token (or the whole thing), minus
    //    every bracket group, then whitespace-collapsed.
    let title_end = release.marker_start.map(|pos| pos.min(name.len())).unwrap_or(name.len());
    let mut title = name[title_start.min(title_end)..title_end].to_string();
    if release.episode.is_none() && release.is_batch {
        if let Some(caps) = batch_range().captures(&title) {
            if let Some(m) = caps.get(0) {
                title.replace_range(m.range(), " ");
            }
        }
    }
    release.title = clean_title(&title);
    release.normalized_title = normalize(&release.title);

    // 6. Languages live in metadata brackets only.
    release.languages = extract_languages(name);
    release.dual_audio = contains_dual_audio(name);

    release
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(name: &str) -> Release {
        parse_release(name)
    }

    #[test]
    fn parses_a_standard_single_episode() {
        let r = parse("[SubsPlease] Sousou no Frieren - 28 (1080p) [ABCD1234].mkv");
        assert_eq!(r.group.as_deref(), Some("SubsPlease"));
        assert_eq!(r.title, "Sousou no Frieren");
        assert_eq!(r.normalized_title, "sousou no frieren");
        assert_eq!(r.episode, Some(28.0));
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.crc.as_deref(), Some("ABCD1234"));
        assert_eq!(r.extension.as_deref(), Some("mkv"));
        assert!(!r.is_batch);
    }

    #[test]
    fn parses_season_episode_form() {
        let r = parse("[Erai-raws] Sousou no Frieren - S01E05 (1080p) [ABCD1234].mkv");
        assert_eq!(r.group.as_deref(), Some("Erai-raws"));
        assert_eq!(r.season, Some(1));
        assert_eq!(r.episode, Some(5.0));
        assert_eq!(r.title, "Sousou no Frieren");
    }

    #[test]
    fn parses_decimal_episodes() {
        let r = parse("[SubsPlease] Steins;Gate - 6.5 (1080p) [01234567].mkv");
        assert_eq!(r.episode, Some(6.5), "OVAs with decimals survive");
        assert_eq!(r.title, "Steins;Gate");
    }

    #[test]
    fn parses_batches_and_ranges() {
        let r = parse("[Judas] Sousou no Frieren - 01-28 [BD 1080p] [Batch]");
        assert!(r.is_batch, "{r:?}");
        assert_eq!(r.episode, None, "a range is not a single episode");

        let r = parse("[Erai-raws] Frieren 2nd Season [1080p CR WEBRip HEVC AAC][MultiSub] (unofficial batch)");
        assert!(r.is_batch);
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.codec.as_deref(), Some("hevc"));
        assert_eq!(r.source.as_deref(), Some("webrip"));
        assert!(r.languages.contains(&"multisub".to_string()));
    }

    #[test]
    fn parses_movies_without_episode() {
        let r = parse("Kimi no Na wa (2016) [BD1080p] [Dual Audio].mkv");
        assert_eq!(r.episode, None);
        assert_eq!(r.title, "Kimi no Na wa");
        assert_eq!(r.normalized_title, "kimi no na wa");
        assert_eq!(r.year, Some(2016));
        assert!(r.dual_audio);
        assert_eq!(r.resolution.as_deref(), Some("1080p"));
        assert_eq!(r.source.as_deref(), Some("bd"));
    }

    #[test]
    fn sequel_titles_do_not_swallow_their_numbers() {
        let a = parse("[SubsPlease] Kimi no Na wa - 01 (1080p).mkv");
        assert_eq!(a.title, "Kimi no Na wa");

        let b = parse("[Group] Re:Zero kara Hajimeru Isekai Seikatsu - 05 (1080p).mkv");
        assert_eq!(b.title, "Re:Zero kara Hajimeru Isekai Seikatsu");
        assert_eq!(b.episode, Some(5.0));

        let c = parse("[Group] Re:Creators - 05 (1080p).mkv");
        assert_eq!(c.title, "Re:Creators", "a different show entirely");
    }

    #[test]
    fn version_and_codec_and_source() {
        let r = parse("[AUTISM] Sousou no Frieren - S01v2 (BD Remux 1080p AVC FLAC) [Dual-Audio]");
        assert_eq!(r.version, Some(2), "{r:?}");
        assert_eq!(r.codec.as_deref(), Some("avc"));
        assert_eq!(r.source.as_deref(), Some("remux"));
        assert!(r.dual_audio);

        let r = parse("[Group] Show - 05v3 (720p x265).mkv");
        assert_eq!(r.version, Some(3));
        assert_eq!(r.codec.as_deref(), Some("hevc"));
    }

    #[test]
    fn normalization_follows_the_design() {
        assert_eq!(normalize("Kimi no Na wa."), "kimi no na wa");
        assert_eq!(normalize("Ｓｏｕｓｏｕ ｎｏ Ｆｒｉｅｒｅｎ"), "sousou no frieren");
        assert_eq!(normalize("Frieren: Beyond Journey's End"), "frieren beyond journey s end");
        assert_eq!(normalize("Frieren Season 2"), "frieren 2", "season words stripped");
        assert_eq!(normalize("Frieren 2nd Season"), "frieren 2");
        assert_eq!(normalize("Show - 02"), normalize("Show - 2"), "zero padding ignored");
        assert_eq!(normalize("  KIMI   no  NA wa  "), "kimi no na wa");
    }

    #[test]
    fn fuzzy_ratio_is_strict_where_it_matters() {
        assert_eq!(fuzzy_ratio("kimi no na wa", "kimi no na wa"), 1.0);
        assert!(fuzzy_ratio("kimi no na wa", "kimi no na wb") > 0.92, "typos still clear 0.92");
        assert!(
            fuzzy_ratio("kimi no na wa", "kimi no todoke") < 0.7,
            "a sequel sharing a prefix must not score as a match"
        );
        assert!(fuzzy_ratio("", "") == 1.0);
        assert!(fuzzy_ratio("abc", "") == 0.0);
    }

    #[test]
    fn metadata_brackets_are_not_group_names() {
        let r = parse("[1080p] Some Show - 01.mkv");
        assert_eq!(r.group, None, "a resolution is not a fansub group");
        let r = parse("[ABCD1234] Some Show - 01.mkv");
        assert_eq!(r.group, None, "a CRC is not a fansub group");
    }

    #[test]
    fn parses_the_real_feed_titles() {
        let xml = include_str!("../../tests/fixtures/nyaa_frieren.xml");
        let items = crate::nyaa::parse_feed(xml).unwrap();
        let mut parsed = 0;
        for item in &items {
            let r = parse(&item.title);
            if !r.title.is_empty() {
                parsed += 1;
            }
            assert!(
                !r.title.contains('['),
                "brackets stripped from title: {:?}",
                r.title
            );
            assert!(r.title.len() <= item.title.len(), "tags removed or kept");
        }
        assert!(parsed >= items.len() * 9 / 10, "most real titles parse");
    }
}
