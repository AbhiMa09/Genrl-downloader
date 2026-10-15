use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicI64, Ordering};

use crate::error::Result;
use crate::models::{Job, JobState};

/// How often the monitor step re-reads qBittorrent.
pub const MONITOR_POLL_SECS: i64 = 10;

/// qBittorrent's category for everything this app adds.
pub const CATEGORY: &str = "genrl";

/// Where disk facts come from.
pub trait DiskProbe: Send + Sync {
    /// Free bytes on the filesystem holding `folder`; `None` when unknown.
    fn free_bytes(&self, folder: &Path) -> Option<u64>;
}

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsDiskProbe;

impl DiskProbe for FsDiskProbe {
    fn free_bytes(&self, folder: &Path) -> Option<u64> {
        if !folder.exists() {
            return None;
        }
        fs2::available_space(folder).ok()
    }
}

/// A probe tests control.
#[derive(Debug, Default)]
pub struct FakeDiskProbe {
    free: AtomicI64,
}

impl FakeDiskProbe {
    pub fn new(free_bytes: u64) -> Self {
        FakeDiskProbe {
            free: AtomicI64::new(free_bytes.min(i64::MAX as u64) as i64),
        }
    }

    pub fn set(&self, free_bytes: u64) {
        self.free.store(free_bytes.min(i64::MAX as u64) as i64, Ordering::SeqCst);
    }
}

impl DiskProbe for FakeDiskProbe {
    fn free_bytes(&self, _folder: &Path) -> Option<u64> {
        let free = self.free.load(Ordering::SeqCst);
        if free < 0 {
            None
        } else {
            Some(free as u64)
        }
    }
}

/// What the library step does with a finished download.
///
/// Phase 6 implements this (rename/move, synopsis, poster, folder icon). Until
/// then [`NoopFinalizer`] leaves the file where qBittorrent saved it, and the
/// queue stops at "completed, file verified".
#[derive(Debug, Clone, PartialEq)]
pub struct FinalizeSuccess {
    pub destination_path: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FinalizeOutcome {
    Done(FinalizeSuccess),
    NeedsInput { question_id: i64 },
}

pub trait Finalizer: Send + Sync {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>>;
}

/// Everything the library step needs, owned by the caller.
pub struct FinalizeContext<'a> {
    pub job: &'a Job,
    pub episode_id: i64,
    pub mal_id: i64,
    /// Where qBittorrent put the file.
    pub content_path: &'a str,
    pub size_bytes: i64,
}

/// The phase-4 finalizer: verify, then stop.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopFinalizer;

impl Finalizer for NoopFinalizer {
    fn finalize<'a>(
        &'a self,
        context: &'a FinalizeContext<'_>,
    ) -> Pin<Box<dyn Future<Output = Result<FinalizeOutcome>> + Send + 'a>> {
        let dest = context.content_path.to_string();
        Box::pin(async move {
            Ok(FinalizeOutcome::Done(FinalizeSuccess {
                destination_path: dest,
            }))
        })
    }
}

/// One step's outcome, for the scheduler's report and the tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepOutcome {
    pub job_id: i64,
    pub from: JobState,
    pub to: JobState,
}

/// The context a step works in.
#[derive(Debug, Clone)]
pub(crate) struct JobContext {
    pub(crate) mal_id: i64,
    pub(crate) number: f64,
    pub(crate) title: String,
}

/// The torrent a job chose.
#[derive(Debug, Clone)]
pub(crate) struct ChosenTorrent {
    pub(crate) info_hash: String,
    pub(crate) name: String,
    pub(crate) magnet: String,
    pub(crate) size_bytes: i64,
}
