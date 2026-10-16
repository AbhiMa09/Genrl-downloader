use anyhow::{Context, Result};
use genrl_core::Db;
use std::path::Path;

pub fn open_db(data_dir: &Path) -> Result<Db> {
    let path = data_dir.join("genrl.db");
    Db::open(&path).with_context(|| format!("opening database at {}", path.display()))
}

/// UTC stamp for a Unix timestamp: `2026-10-12 18:59:03`.
pub fn chrono_lite(unix: i64) -> String {
    // Howard Hinnant's civil_from_days.
    let days = unix.div_euclid(86_400);
    let secs = unix.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// `1536` → `1.5 MiB`, matching the feed's own size strings.
pub fn human_bytes(bytes: u64) -> String {
    const MIB: u64 = 1 << 20;
    const GIB: u64 = 1 << 30;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= 1024 {
        format!("{:.0} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

/// qBittorrent's ETA in a human form; `8640000` means "unknown".
pub fn fmt_eta(seconds: i64) -> String {
    if seconds <= 0 || seconds >= 8_640_000 {
        return "-".to_string();
    }
    let (hours, minutes, secs) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    if hours > 0 {
        format!("{hours}h{minutes:02}m")
    } else if minutes > 0 {
        format!("{minutes}m{secs:02}s")
    } else {
        format!("{secs}s")
    }
}

/// Truncate on a character boundary — release names are not always ASCII.
pub fn truncate_text(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_known_timestamps() {
        assert_eq!(chrono_lite(0), "1970-01-01 00:00:00");
        assert_eq!(chrono_lite(1_760_000_000), "2025-10-09 08:53:20");
    }

    #[test]
    fn formats_sizes_and_etas() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(2048), "2 KiB");
        assert_eq!(human_bytes(1_600_000_000), "1.5 GiB");
        assert_eq!(human_bytes(5 * 1024 * 1024), "5.0 MiB");

        assert_eq!(fmt_eta(8_640_000), "-", "qBittorrent's unknown");
        assert_eq!(fmt_eta(0), "-");
        assert_eq!(fmt_eta(45), "45s");
        assert_eq!(fmt_eta(125), "2m05s");
        assert_eq!(fmt_eta(7322), "2h02m");
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_text("short", 50), "short");
        assert_eq!(truncate_text("frieren", 3), "fri…");
        assert_eq!(truncate_text("素の確認テスト", 4), "素…");
    }
}
