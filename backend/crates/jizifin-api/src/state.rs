//! Shared application state.

use jizifin_auditor::Auditor;
use jizifin_store::{Database, DuckSnapshotLoader, StoreError};
use std::path::Path;
use std::sync::Arc;

/// Handles cloned into every request.
#[derive(Clone)]
pub struct AppState {
    /// Database handle (used by CRUD routes).
    #[allow(dead_code)]
    pub db: Database,
    /// Owner of contentious calculations.
    pub auditor: Arc<Auditor>,
}

impl AppState {
    /// Opens the database and wires the Auditor to it.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let db = Database::open(path)?;
        let loader = Arc::new(DuckSnapshotLoader::new(db.clone()));
        let auditor = Arc::new(Auditor::new(loader, db.version()));
        Ok(Self { db, auditor })
    }
}
