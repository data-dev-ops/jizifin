//! DuckDB 2.0 persistence: connection pool, migrations, repositories and the Auditor snapshot loader.

mod database;
mod error;
mod loader;
mod migrate;
mod rows;
mod sql;

pub mod inputs;
pub mod repo;

pub use database::Database;
pub use duckdb::Connection;
pub use error::StoreError;
pub use loader::DuckSnapshotLoader;
