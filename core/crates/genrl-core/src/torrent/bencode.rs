use crate::error::{Error, Result};
use sha1::{Digest, Sha1};

pub const MAX_TORRENT_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentInfo {
    pub info_hash: String,
    pub name: Option<String>,
    pub is_v2: bool,
}

pub fn parse_torrent_info_hash(bytes: &[u8]) -> Result<TorrentInfo> {
    if bytes.len() > MAX_TORRENT_SIZE {
        return Err(Error::Other("torrent file exceeds 10 MB limit".into()));
    }
    if bytes.is_empty() || bytes[0] != b'd' {
        return Err(Error::Other(
            "not a bencode dictionary (rejected HTML or invalid format)".into(),
        ));
    }

    let mut pos = 1;
    let mut info_slice: Option<&[u8]> = None;

    while pos < bytes.len() && bytes[pos] != b'e' {
        let key = read_string(bytes, &mut pos)?;
        if key == b"info" {
            let start = pos;
            skip_item(bytes, &mut pos, 1)?;
            info_slice = Some(&bytes[start..pos]);
        } else {
            skip_item(bytes, &mut pos, 1)?;
        }
    }

    let info_bytes =
        info_slice.ok_or_else(|| Error::Other("torrent missing info dictionary".into()))?;
    if info_bytes.is_empty() || info_bytes[0] != b'd' {
        return Err(Error::Other("info value is not a dictionary".into()));
    }

    let mut has_pieces = false;
    let mut has_file_tree = false;
    let mut name = None;

    let mut ipos = 1;
    while ipos < info_bytes.len() && info_bytes[ipos] != b'e' {
        let key = read_string(info_bytes, &mut ipos)?;
        match key {
            b"name" => {
                if let Ok(val) = read_string(info_bytes, &mut ipos) {
                    name = String::from_utf8(val.to_vec()).ok();
                }
            }
            b"pieces" => {
                has_pieces = true;
                skip_item(info_bytes, &mut ipos, 1)?;
            }
            b"file tree" => {
                has_file_tree = true;
                skip_item(info_bytes, &mut ipos, 1)?;
            }
            _ => skip_item(info_bytes, &mut ipos, 1)?,
        }
    }

    if has_pieces {
        let mut hasher = Sha1::new();
        hasher.update(info_bytes);
        let info_hash = format!("{:x}", hasher.finalize());
        Ok(TorrentInfo {
            info_hash,
            name,
            is_v2: has_file_tree,
        })
    } else if has_file_tree {
        Err(Error::Other("v2-only torrents not supported".into()))
    } else {
        Err(Error::Other(
            "invalid torrent: info dict missing required fields".into(),
        ))
    }
}

fn read_string<'a>(b: &'a [u8], pos: &mut usize) -> Result<&'a [u8]> {
    let colon = b
        .get(*pos..)
        .and_then(|s| s.iter().position(|&c| c == b':'))
        .ok_or_else(|| Error::Other("malformed string in bencode".into()))?;
    let len_str = std::str::from_utf8(&b[*pos..*pos + colon])
        .map_err(|_| Error::Other("invalid string length".into()))?;
    let len: usize = len_str
        .parse()
        .map_err(|_| Error::Other("non-numeric string length".into()))?;
    let start = *pos + colon + 1;
    let end = start
        .checked_add(len)
        .ok_or_else(|| Error::Other("string length overflow".into()))?;
    if end > b.len() {
        return Err(Error::Other("string extends past EOF".into()));
    }
    *pos = end;
    Ok(&b[start..end])
}

fn skip_item(b: &[u8], pos: &mut usize, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Err(Error::Other("bencode nesting depth exceeded".into()));
    }
    let tag = *b
        .get(*pos)
        .ok_or_else(|| Error::Other("unexpected EOF".into()))?;
    match tag {
        b'i' => {
            *pos += 1;
            let end = b
                .get(*pos..)
                .and_then(|s| s.iter().position(|&c| c == b'e'))
                .ok_or_else(|| Error::Other("unterminated integer".into()))?;
            *pos += end + 1;
            Ok(())
        }
        b'0'..=b'9' => {
            read_string(b, pos)?;
            Ok(())
        }
        b'l' => {
            *pos += 1;
            while *pos < b.len() && b[*pos] != b'e' {
                skip_item(b, pos, depth + 1)?;
            }
            if *pos >= b.len() {
                return Err(Error::Other("unterminated list".into()));
            }
            *pos += 1;
            Ok(())
        }
        b'd' => {
            *pos += 1;
            while *pos < b.len() && b[*pos] != b'e' {
                read_string(b, pos)?;
                skip_item(b, pos, depth + 1)?;
            }
            if *pos >= b.len() {
                return Err(Error::Other("unterminated dictionary".into()));
            }
            *pos += 1;
            Ok(())
        }
        _ => Err(Error::Other(format!("invalid bencode byte: {tag}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_fixture_and_known_info_hash() {
        let info = b"d4:name10:sample.mkv6:pieces20:12345678901234567890e";
        let mut torrent = b"d8:announce17:http://tracker.io4:info".to_vec();
        torrent.extend_from_slice(info);
        torrent.push(b'e');

        let mut hasher = Sha1::new();
        hasher.update(info);
        let expected_hash = format!("{:x}", hasher.finalize());

        let res = parse_torrent_info_hash(&torrent).expect("must parse valid torrent");
        assert_eq!(res.info_hash, expected_hash);
        assert_eq!(res.name.as_deref(), Some("sample.mkv"));
        assert!(!res.is_v2);
    }

    #[test]
    fn test_rejects_html_and_non_dict() {
        assert!(
            parse_torrent_info_hash(b"<!DOCTYPE html><html><body>Error</body></html>").is_err()
        );
        assert!(parse_torrent_info_hash(b"li123ee").is_err());
        assert!(parse_torrent_info_hash(b"").is_err());
    }

    #[test]
    fn test_truncated_torrent_rejected() {
        assert!(parse_torrent_info_hash(b"d4:info").is_err());
        assert!(parse_torrent_info_hash(b"d4:infod4:name4:teste").is_err());
    }

    #[test]
    fn test_info_not_first_key_and_nested_structures() {
        let info =
            b"d5:filesld6:lengthi100e4:pathl4:fileeee4:name4:test6:pieces20:12345678901234567890e";
        let mut torrent = b"d7:comment7:testing4:info".to_vec();
        torrent.extend_from_slice(info);
        torrent.extend_from_slice(b"10:created by5:genrle");

        let mut hasher = Sha1::new();
        hasher.update(info);
        let expected = format!("{:x}", hasher.finalize());

        let res = parse_torrent_info_hash(&torrent).unwrap();
        assert_eq!(res.info_hash, expected);
        assert_eq!(res.name.as_deref(), Some("test"));
    }

    #[test]
    fn test_v2_only_and_hybrid_handling() {
        let v2_only = b"d4:infod9:file tree4:tree4:name4:testee";
        let err = parse_torrent_info_hash(v2_only).unwrap_err();
        assert!(err.to_string().contains("v2-only"));

        let hybrid = b"d4:infod9:file tree4:tree4:name4:test6:pieces20:12345678901234567890ee";
        let res = parse_torrent_info_hash(hybrid).unwrap();
        assert!(res.is_v2);
    }

    #[test]
    fn test_missing_info_dict_rejected() {
        let err = parse_torrent_info_hash(b"d4:name4:teste").unwrap_err();
        assert!(err.to_string().contains("missing info dictionary"));
    }

    #[test]
    fn test_exceeds_10mb_rejected() {
        let huge = vec![b'd'; MAX_TORRENT_SIZE + 1];
        let err = parse_torrent_info_hash(&huge).unwrap_err();
        assert!(err.to_string().contains("10 MB limit"));
    }

    #[test]
    fn test_nesting_depth_exceeded_rejected() {
        let mut deeply_nested = b"d4:info".to_vec();
        for _ in 0..35 {
            deeply_nested.push(b'd');
            deeply_nested.extend_from_slice(b"1:a");
        }
        deeply_nested.extend_from_slice(b"i1e");
        for _ in 0..35 {
            deeply_nested.push(b'e');
        }
        deeply_nested.push(b'e');
        let err = parse_torrent_info_hash(&deeply_nested).unwrap_err();
        assert!(err.to_string().contains("nesting depth"));
    }

    #[test]
    fn test_fuzz_garbage_never_panics() {
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"d4:infoi123ee".to_vec(),
            b"d4:info4:teste".to_vec(),
            b"d4:info".to_vec(),
            b"i-001e".to_vec(),
            b"i99999999999999999999999999999999e".to_vec(),
            b"9999999999999999999999999:abc".to_vec(),
            b"-5:hello".to_vec(),
            b"d-1:x".to_vec(),
            b"d".to_vec(),
            b"de".to_vec(),
            b"l".to_vec(),
            b"le".to_vec(),
            b"i0e".to_vec(),
            b"ie".to_vec(),
            b"d1:a".to_vec(),
            b"l1:a".to_vec(),
            b"d1:ai1e".to_vec(),
            b"d4:infod6:pieces20:shortee".to_vec(),
            b"d4:infod6:piecese".to_vec(),
            b"\x00\xff\xfe\xfd".to_vec(),
        ];
        for i in 1..=10 {
            cases.push((0..i * 13).map(|b| (b * 47 % 255) as u8).collect());
        }
        assert!(cases.len() >= 30);
        for bytes in &cases {
            let _ = parse_torrent_info_hash(bytes);
        }
    }
}
