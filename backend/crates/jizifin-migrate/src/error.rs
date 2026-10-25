//! Migration failures. Any failure rolls the whole migration back.

use thiserror::Error;

/// Why the migration stopped.
#[derive(Debug, Error)]
pub enum MigrateError {
    /// Bad arguments or environment.
    #[error("{0}")]
    Usage(String),
    /// Legacy SQLite read failed.
    #[error("legacy read: {0}")]
    Legacy(#[from] rusqlite::Error),
    /// New store rejected a row.
    #[error("store: {0}")]
    Store(#[from] jizifin_store::StoreError),
    /// Auditor refused to seal a month.
    #[error("audit: {0}")]
    Audit(#[from] jizifin_auditor::AuditError),
    /// Legacy data cannot be mapped.
    #[error("legacy data: {0}")]
    Data(String),
}

impl From<jizifin_domain::DomainError> for MigrateError {
    fn from(e: jizifin_domain::DomainError) -> Self {
        Self::Data(e.to_string())
    }
}
