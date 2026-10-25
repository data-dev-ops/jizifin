//! Store failures, classified so the API can map them to HTTP statuses.

use thiserror::Error;

/// Anything that can go wrong talking to DuckDB.
#[derive(Debug, Error)]
pub enum StoreError {
    /// DuckDB driver error.
    #[error("duckdb: {0}")]
    Duck(duckdb::Error),
    /// Connection pool error.
    #[error("pool: {0}")]
    Pool(#[from] r2d2::Error),
    /// Row does not exist.
    #[error("{0} not found")]
    NotFound(String),
    /// Uniqueness, reference or sealed-period conflict.
    #[error("{0}")]
    Conflict(String),
    /// Input violates a business rule.
    #[error("{0}")]
    Invalid(String),
}

impl From<duckdb::Error> for StoreError {
    fn from(e: duckdb::Error) -> Self {
        let text = e.to_string();
        match text.contains("Constraint Error") {
            true => Self::Conflict(text),
            false => Self::Duck(e),
        }
    }
}

impl From<jizifin_domain::DomainError> for StoreError {
    fn from(e: jizifin_domain::DomainError) -> Self {
        Self::Invalid(e.to_string())
    }
}
