use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// Find the nearest existing ancestor for a path (or the path itself if it exists).
pub fn nearest_existing_ancestor(path: &Path) -> Result<PathBuf> {
    let mut current = path.to_path_buf();
    while !current.exists() {
        if !current.pop() {
            return Err(Error::Other(format!(
                "no existing ancestor found for path: {}",
                path.display()
            )));
        }
    }
    Ok(current)
}

#[cfg(unix)]
fn get_volume_id(path: &Path) -> Result<u64> {
    use std::os::unix::fs::MetadataExt;
    let existing = nearest_existing_ancestor(path)?;
    let meta = std::fs::metadata(&existing).map_err(|e| {
        Error::Other(format!(
            "failed to read metadata for {}: {e}",
            existing.display()
        ))
    })?;
    Ok(meta.dev())
}

#[cfg(windows)]
fn get_volume_id(path: &Path) -> Result<u64> {
    use std::os::windows::fs::MetadataExt;
    let existing = nearest_existing_ancestor(path)?;
    let file = std::fs::File::open(&existing)
        .map_err(|e| Error::Other(format!("failed to open {}: {e}", existing.display())))?;
    let meta = file.metadata().map_err(|e| {
        Error::Other(format!(
            "failed to read metadata for {}: {e}",
            existing.display()
        ))
    })?;
    Ok(meta.volume_serial_number().map(|v| v as u64).unwrap_or(0))
}

#[cfg(not(any(unix, windows)))]
fn get_volume_id(_path: &Path) -> Result<u64> {
    Ok(0)
}

/// Check if two paths reside on the same volume/filesystem.
pub fn same_volume(a: &Path, b: &Path) -> Result<bool> {
    same_volume_with(a, b, get_volume_id)
}

/// Check if two paths reside on the same volume using an injectable volume resolver.
pub fn same_volume_with<F>(a: &Path, b: &Path, mut resolver: F) -> Result<bool>
where
    F: FnMut(&Path) -> Result<u64>,
{
    let vol_a = resolver(a)?;
    let vol_b = resolver(b)?;
    Ok(vol_a == vol_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_volume_same_dir() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("a.mkv");
        let file_b = dir.path().join("b.mkv");
        std::fs::write(&file_a, b"1").unwrap();
        std::fs::write(&file_b, b"2").unwrap();

        assert!(same_volume(&file_a, &file_b).unwrap());
    }

    #[test]
    fn test_same_volume_nonexistent_target_uses_ancestor() {
        let dir = tempfile::tempdir().unwrap();
        let file_a = dir.path().join("source.mkv");
        std::fs::write(&file_a, b"data").unwrap();

        let non_existent = dir.path().join("nested").join("sub").join("target.mkv");
        assert!(!non_existent.exists());

        // Uses nearest existing ancestor (dir.path())
        assert!(same_volume(&file_a, &non_existent).unwrap());
    }

    #[test]
    fn test_same_volume_with_injected_resolver() {
        let p1 = Path::new("/mnt/drive1/movie.mkv");
        let p2 = Path::new("/mnt/drive2/movie.mkv");

        let res = same_volume_with(p1, p2, |p| {
            if p.starts_with("/mnt/drive1") {
                Ok(1001)
            } else {
                Ok(2002)
            }
        })
        .unwrap();
        assert!(!res);

        let res_same = same_volume_with(p1, p1, |_| Ok(1001)).unwrap();
        assert!(res_same);
    }

    #[test]
    fn test_same_volume_error_mapping() {
        let p1 = Path::new("/fake/path/a");
        let p2 = Path::new("/fake/path/b");

        let err =
            same_volume_with(p1, p2, |_| Err(Error::Other("disk read error".into()))).unwrap_err();

        assert!(err.to_string().contains("disk read error"));
    }
}
