//! Debug logging that rotates by size.
//!
//! The Activity page reads the `events` table; this file log is the optional
//! developer switch from Settings. Rotation keeps `genrl.log` under
//! `log_max_bytes` and retains `log_keep` older copies.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Full path of the log file, e.g. `~/.local/share/genrl-downloader/genrl.log`.
    pub path: PathBuf,
    pub max_bytes: u64,
    pub keep: usize,
    /// tracing filter, e.g. `info` or `genrl_core=debug`.
    pub level: String,
    /// Also echo to stderr (useful when running from a terminal).
    pub stderr: bool,
}

impl LogConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        LogConfig {
            path: path.into(),
            max_bytes: 5 * 1024 * 1024,
            keep: 3,
            level: "info".to_string(),
            stderr: true,
        }
    }
}

struct RotState {
    file: File,
    path: PathBuf,
    written: u64,
    max_bytes: u64,
    keep: usize,
}

impl RotState {
    fn open(path: &Path, max_bytes: u64, keep: usize) -> std::io::Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let file = open_log(path)?;
        let written = file.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(RotState {
            file,
            path: path.to_path_buf(),
            written,
            max_bytes,
            keep,
        })
    }

    fn rotate(&mut self) -> std::io::Result<()> {
        if self.keep == 0 {
            self.file = truncate(&self.path)?;
            self.written = 0;
            return Ok(());
        }
        // genrl.log.(keep-1) -> dropped, ... genrl.log.1 -> genrl.log.2
        for i in (1..self.keep).rev() {
            let from = numbered(&self.path, i);
            let to = numbered(&self.path, i + 1);
            if from.exists() {
                let _ = std::fs::rename(&from, &to);
            }
        }
        let _ = std::fs::rename(&self.path, numbered(&self.path, 1));
        self.file = open_log(&self.path)?;
        self.written = 0;
        Ok(())
    }
}

/// A size-rotating log file, safe to share between threads.
#[derive(Clone)]
pub struct RotatingWriter {
    state: Arc<Mutex<RotState>>,
}

impl RotatingWriter {
    pub fn open(config: &LogConfig) -> Result<Self> {
        let state = RotState::open(&config.path, config.max_bytes, config.keep)?;
        Ok(RotatingWriter {
            state: Arc::new(Mutex::new(state)),
        })
    }
}

impl Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.written > 0 && state.written + buf.len() as u64 > state.max_bytes {
            state.rotate()?;
        }
        let n = state.file.write(buf)?;
        state.written += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.file.flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for RotatingWriter {
    type Writer = RotatingWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Writes to every sink: the rotating file and, optionally, stderr.
#[derive(Clone)]
struct MultiWriter {
    sinks: Arc<Vec<Mutex<Box<dyn Write + Send + Sync>>>>,
}

impl Write for MultiWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for sink in self.sinks.iter() {
            if let Ok(mut sink) = sink.lock() {
                let _ = sink.write(buf);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        for sink in self.sinks.iter() {
            if let Ok(mut sink) = sink.lock() {
                let _ = sink.flush();
            }
        }
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for MultiWriter {
    type Writer = MultiWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Install the global tracing subscriber.
///
/// `RUST_LOG` wins when set; otherwise `config.level` is used.
pub fn init(config: &LogConfig) -> Result<()> {
    use tracing_subscriber::EnvFilter;

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(config.level.clone()));

    let mut sinks: Vec<Mutex<Box<dyn Write + Send + Sync>>> =
        vec![Mutex::new(Box::new(RotatingWriter::open(config)?))];
    if config.stderr {
        sinks.push(Mutex::new(Box::new(std::io::stderr())));
    }
    let writer = MultiWriter {
        sinks: Arc::new(sinks),
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);
    Ok(())
}

fn numbered(path: &Path, n: usize) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "genrl.log".to_string());
    name.push_str(&format!(".{n}"));
    path.with_file_name(name)
}

fn open_log(path: &Path) -> std::io::Result<File> {
    let file = OpenOptions::new().create(true).append(true).open(path)?;
    set_owner_only(path);
    Ok(file)
}

fn truncate(path: &Path) -> std::io::Result<File> {
    let file = File::create(path)?;
    set_owner_only(path);
    Ok(file)
}

#[cfg(unix)]
fn set_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mode = meta.permissions().mode();
        if mode & 0o077 != 0 {
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

#[cfg(not(unix))]
fn set_owner_only(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_bytes(writer: &mut RotatingWriter, len: usize) {
        let chunk = vec![b'x'; len];
        writer.write_all(&chunk).unwrap();
    }

    #[test]
    fn rotates_when_the_file_gets_full() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genrl.log");
        let config = LogConfig {
            path: path.clone(),
            max_bytes: 1000,
            keep: 2,
            level: "info".into(),
            stderr: false,
        };
        let mut writer = RotatingWriter::open(&config).unwrap();
        write_bytes(&mut writer, 600);
        write_bytes(&mut writer, 600); // exceeds 1000 -> rotate

        assert!(path.exists(), "current log recreated");
        assert!(numbered(&path, 1).exists(), "previous log kept");

        write_bytes(&mut writer, 600); // rotate again -> .1 becomes .2
        assert!(numbered(&path, 2).exists());
        assert!(
            numbered(&path, 3).exists() == false,
            "keep = 2 caps the chain"
        );
    }

    #[test]
    fn respects_keep_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("genrl.log");
        let config = LogConfig {
            path: path.clone(),
            max_bytes: 100,
            keep: 0,
            level: "info".into(),
            stderr: false,
        };
        let mut writer = RotatingWriter::open(&config).unwrap();
        write_bytes(&mut writer, 80);
        write_bytes(&mut writer, 80);
        assert!(!numbered(&path, 1).exists());
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size <= 100, "truncated instead of rotated: {size}");
    }
}
