//! Error type shared by the core.
//!
//! The queue layer maps these onto job states: a transient [`Error::Http`] or
//! [`Error::Timeout`] retries with the backoff rule, while [`Error::BreakerOpen`]
//! parks the job in `blocked` until the service recovers.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("request to {url} timed out after {timeout_ms} ms")]
    Timeout { url: String, timeout_ms: u64 },

    #[error("http {status} from {url}")]
    Status {
        status: u16,
        url: String,
        /// Value of `Retry-After`, if the server sent one.
        retry_after: Option<std::time::Duration>,
        /// Response body, if read.
        body: Option<String>,
    },

    #[error("rate limited by {service} (429); retry after {retry_after:?}")]
    RateLimited {
        service: String,
        retry_after: Option<std::time::Duration>,
    },

    #[error("circuit breaker open for {service}; next probe in {retry_in_secs}s")]
    BreakerOpen { service: String, retry_in_secs: u64 },

    #[error("response from {url} is {got} bytes, over the {cap} byte cap")]
    BodyTooLarge { url: String, got: usize, cap: usize },

    #[error("insecure permissions on {path}: {mode:o} (must not be group/world writable)")]
    InsecurePermissions { path: String, mode: u32 },

    #[error("invalid url: {0}")]
    InvalidUrl(String),

    #[error("{service} authentication failed: {message}")]
    Auth { service: String, message: String },

    #[error("not found: {0}")]
    NotFound(String),

    /// A move the state machine's table does not allow.
    #[error("illegal job transition {from} → {to} (job {job_id})")]
    IllegalTransition {
        from: crate::models::JobState,
        to: crate::models::JobState,
        job_id: i64,
    },

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// True when the failure is worth retrying after a delay.
    pub fn is_transient(&self) -> bool {
        match self {
            Error::Timeout { .. } | Error::RateLimited { .. } | Error::BreakerOpen { .. } => true,
            Error::Status { status, .. } => (500..=599).contains(status),
            Error::Http(e) => e.is_connect() || e.is_timeout() || e.is_request(),
            _ => false,
        }
    }

    /// True when the failure was a timeout rather than a refused connection.
    pub fn is_timeout(&self) -> bool {
        match self {
            Error::Timeout { .. } => true,
            Error::Http(e) => e.is_timeout(),
            _ => false,
        }
    }

    /// True when the far end could not be reached at all — refused, no route,
    /// DNS failure or timeout. The health check uses this to decide whether to
    /// offer starting `qbittorrent-nox`.
    pub fn is_connection_failure(&self) -> bool {
        match self {
            Error::Timeout { .. } => true,
            Error::Http(e) => e.is_connect() || e.is_timeout(),
            _ => false,
        }
    }
}
