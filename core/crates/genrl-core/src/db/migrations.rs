//! Versioned schema migrations.
//!
//! Migrations run in order inside a transaction and are recorded in
//! `schema_migrations`, so a half-applied schema cannot survive a crash.

pub struct Migration {
    pub version: i64,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/001_init.sql"),
}];
