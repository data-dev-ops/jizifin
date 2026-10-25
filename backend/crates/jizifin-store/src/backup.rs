//! Whole-database backup as one DuckDB file. Only ciphertext and numbers leave the server;
//! the server never needs the passphrase.

use crate::migrate::LATEST_VERSION;
use crate::schema::{self, TABLES};
use crate::{Database, StoreError};
use duckdb::{Connection, Transaction};
use std::path::Path;

/// Catalog alias used while a backup file is attached.
const ALIAS: &str = "jizifin_backup";

/// Export and restore of complete instances.
pub struct Backup;

impl Backup {
    /// Writes the full ledger (without sessions) into a new DuckDB file at `path`.
    pub fn export(db: &Database, path: &Path) -> Result<(), StoreError> {
        let conn = db.connection()?;
        Self::attach(&conn, path, false)?;
        let result = Self::copy_out(&conn);
        Self::detach(&conn);
        result
    }

    /// Replaces every ledger row with the contents of the backup at `path`.
    pub fn restore(db: &Database, path: &Path) -> Result<(), StoreError> {
        let mut conn = db.connection()?;
        Self::attach(&conn, path, true)?;
        let result = Self::check_version(&conn).and_then(|()| Self::copy_in(&mut conn));
        Self::detach(&conn);
        result?;
        db.version().bump();
        Ok(())
    }

    fn attach(conn: &Connection, path: &Path, read_only: bool) -> Result<(), StoreError> {
        let quoted = path.display().to_string().replace('\'', "''");
        let mode = if read_only { " (READ_ONLY)" } else { "" };
        conn.execute_batch(&format!("ATTACH '{quoted}' AS {ALIAS}{mode}"))?;
        Ok(())
    }

    fn detach(conn: &Connection) {
        let _ = conn.execute_batch(&format!("DETACH {ALIAS}"));
    }

    fn copy_out(conn: &Connection) -> Result<(), StoreError> {
        let current: String = conn.query_row("SELECT current_database()", [], |r| r.get(0))?;
        conn.execute_batch(&format!("COPY FROM DATABASE \"{current}\" TO {ALIAS}; DELETE FROM {ALIAS}.sessions;"))?;
        Ok(())
    }

    fn check_version(conn: &Connection) -> Result<(), StoreError> {
        let sql = format!("SELECT COALESCE(MAX(version), 0) FROM {ALIAS}.schema_version");
        let version: i64 = conn.query_row(&sql, [], |r| r.get(0))?;
        match version == LATEST_VERSION {
            true => Ok(()),
            false => Err(StoreError::Invalid(format!("backup schema v{version} does not match server schema v{LATEST_VERSION}"))),
        }
    }

    fn copy_in(conn: &mut Connection) -> Result<(), StoreError> {
        let tx = conn.transaction()?;
        schema::clear(&tx)?;
        TABLES.iter().try_for_each(|(table, sequence)| Self::copy_table(&tx, table, *sequence))?;
        tx.commit()?;
        Ok(())
    }

    fn copy_table(tx: &Transaction<'_>, table: &str, sequence: Option<&str>) -> Result<(), StoreError> {
        tx.execute(&format!("INSERT INTO {table} SELECT * FROM {ALIAS}.{table}"), [])?;
        match sequence {
            Some(seq) => Self::advance(tx, table, seq),
            None => Ok(()),
        }
    }

    /// DuckDB has no `setval`: draw the sequence past the highest restored id.
    fn advance(tx: &Transaction<'_>, table: &str, sequence: &str) -> Result<(), StoreError> {
        let sql = format!("SELECT COUNT(nextval('{sequence}')) FROM range((SELECT COALESCE(MAX(id), 0) FROM {table}))");
        tx.query_row(&sql, [], |r| r.get::<_, i64>(0))?;
        Ok(())
    }
}
