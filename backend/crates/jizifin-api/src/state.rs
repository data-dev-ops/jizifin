//! Shared application state and the blocking-call bridge to DuckDB.

use crate::error::ApiError;
use crate::events::EventHub;
use jizifin_auditor::Auditor;
use jizifin_store::{Connection, Database, DuckSnapshotLoader, StoreError};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Handles cloned into every request.
#[derive(Clone)]
pub struct AppState {
    /// Database handle.
    pub db: Database,
    /// Owner of contentious calculations.
    pub auditor: Arc<Auditor>,
    /// Live event fan-out for `/ws/finance`.
    pub events: EventHub,
    /// Scratch directory for backup files.
    pub tmp_dir: PathBuf,
}

impl AppState {
    /// Opens the database and wires the Auditor to it.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let db = Database::open(path)?;
        let loader = Arc::new(DuckSnapshotLoader::new(db.clone()));
        let auditor = Arc::new(Auditor::new(loader, db.version()));
        let tmp_dir = path.parent().unwrap_or(Path::new(".")).join("tmp");
        Ok(Self { db, auditor, events: EventHub::default(), tmp_dir })
    }

    /// Runs a read off the async runtime.
    pub async fn read<T, F>(&self, work: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let db = self.db.clone();
        blocking(move || db.read(work)).await
    }

    /// Runs a ledger write in one transaction, then announces `topic` to WebSocket clients.
    pub async fn write<T, F>(&self, topic: &'static str, work: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let db = self.db.clone();
        let value = blocking(move || db.write(|tx| work(tx))).await?;
        self.events.publish(topic);
        Ok(value)
    }

    /// Runs a session bookkeeping write that never invalidates audits.
    pub async fn write_meta<T, F>(&self, work: F) -> Result<T, ApiError>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> Result<T, StoreError> + Send + 'static,
    {
        let db = self.db.clone();
        blocking(move || db.write_meta(|tx| work(tx))).await
    }
}

/// Runs `work` on the blocking pool and flattens join errors.
pub async fn blocking<T, E>(work: impl FnOnce() -> Result<T, E> + Send + 'static) -> Result<T, ApiError>
where
    T: Send + 'static,
    E: Into<ApiError> + Send + 'static,
{
    let joined = tokio::task::spawn_blocking(work).await;
    joined.map_err(|e| ApiError::Internal(e.to_string()))?.map_err(Into::into)
}
