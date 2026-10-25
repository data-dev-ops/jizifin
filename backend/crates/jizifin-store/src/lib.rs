//! DuckDB 2.0 persistence: connection pool, migrations and the Auditor snapshot loader.

mod database;
mod error;
mod loader;
mod migrate;

pub use database::Database;
pub use error::StoreError;
pub use loader::DuckSnapshotLoader;
