//! Pooled DuckDB handle. Every committed write bumps the shared data version.

use crate::StoreError;
use crate::migrate::Migrator;
use duckdb::{DuckdbConnectionManager, Transaction};
use jizifin_auditor::DataVersion;
use std::path::Path;
use std::sync::Arc;

type Pool = r2d2::Pool<DuckdbConnectionManager>;
/// A pooled connection.
pub type PooledConnection = r2d2::PooledConnection<DuckdbConnectionManager>;

/// The single owner of the DuckDB file inside this process.
#[derive(Clone)]
pub struct Database {
    pool: Pool,
    version: Arc<DataVersion>,
}

impl Database {
    /// Opens (or creates) the database file and applies pending migrations.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::from_manager(DuckdbConnectionManager::file(path)?)
    }

    /// Opens a private in-memory database with the full schema.
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::from_manager(DuckdbConnectionManager::memory()?)
    }

    /// Version counter shared with the Auditor cache.
    pub fn version(&self) -> Arc<DataVersion> {
        Arc::clone(&self.version)
    }

    /// A pooled connection for reads.
    pub fn connection(&self) -> Result<PooledConnection, StoreError> {
        Ok(self.pool.get()?)
    }

    /// Runs read-only `work` on a pooled connection.
    pub fn read<T>(&self, work: impl FnOnce(&duckdb::Connection) -> Result<T, StoreError>) -> Result<T, StoreError> {
        work(&*self.connection()?)
    }

    /// Runs `work` in one transaction without bumping the data version.
    /// Only for bookkeeping that never feeds a calculation (sessions).
    pub fn write_meta<T>(&self, work: impl FnOnce(&Transaction<'_>) -> Result<T, StoreError>) -> Result<T, StoreError> {
        let mut conn = self.connection()?;
        let tx = conn.transaction()?;
        let value = work(&tx)?;
        tx.commit()?;
        Ok(value)
    }

    /// Runs `work` in one transaction; commits and bumps the data version on success.
    pub fn write<T>(&self, work: impl FnOnce(&Transaction<'_>) -> Result<T, StoreError>) -> Result<T, StoreError> {
        let value = self.write_meta(work)?;
        self.version.bump();
        Ok(value)
    }

    fn from_manager(manager: DuckdbConnectionManager) -> Result<Self, StoreError> {
        let db = Self { pool: Pool::new(manager)?, version: Arc::default() };
        Migrator::new(&db).apply()?;
        Ok(db)
    }
}
