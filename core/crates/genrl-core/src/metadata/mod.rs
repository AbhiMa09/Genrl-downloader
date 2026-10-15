//! Metadata ingestion module: Jikan /v4/anime/{id}/full, MAL API fallback,
//! 7-day caching, local poster downloads, plain text sanitization, and retry with backoff.

pub mod apply;
pub mod batch;
pub mod fetch;
pub mod models;
pub mod poster;

pub use apply::*;
pub use batch::*;
pub use fetch::*;
pub use models::*;
pub use poster::*;
