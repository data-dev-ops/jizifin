//! Store failures.

use thiserror::Error;

/// Anything that can go wrong talking to DuckDB.
#[derive(Debug, Error)]
pub enum StoreError {
    /// DuckDB driver error.
    #[error("duckdb: {0}")]
    Duck(#[from] duckdb::Error),
    /// Connection pool error.
    #[error("pool: {0}")]
    Pool(#[from] r2d2::Error),
}
