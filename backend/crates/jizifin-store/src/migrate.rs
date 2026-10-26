//! Forward-only, numbered SQL migrations tracked in `schema_version`.

use crate::{Database, StoreError};
use duckdb::{Transaction, params};

/// Ordered `(version, sql)` pairs compiled into the binary.
const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("../migrations/0001_init.sql"))];

/// Schema version after every migration ran; backups must match it.
pub const LATEST_VERSION: i64 = 1;

/// Applies migrations newer than the recorded schema version.
pub struct Migrator<'a> {
    db: &'a Database,
}

impl<'a> Migrator<'a> {
    /// Migrator for `db`.
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    /// Applies every pending migration in one transaction.
    pub fn apply(&self) -> Result<(), StoreError> {
        self.db.write(|tx| {
            tx.execute_batch("CREATE TABLE IF NOT EXISTS schema_version (version BIGINT PRIMARY KEY)")?;
            let current = Self::current(tx)?;
            MIGRATIONS.iter().filter(|(v, _)| *v > current).try_for_each(|m| Self::run(tx, *m))
        })
    }

    fn current(tx: &Transaction<'_>) -> Result<i64, StoreError> {
        Ok(tx.query_row("SELECT COALESCE(MAX(version), 0) FROM schema_version", [], |r| r.get(0))?)
    }

    fn run(tx: &Transaction<'_>, (version, sql): (i64, &str)) -> Result<(), StoreError> {
        tx.execute_batch(sql)?;
        tx.execute("INSERT INTO schema_version (version) VALUES (?)", params![version])?;
        Ok(())
    }
}
