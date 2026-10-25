//! Runtime configuration from environment variables.

use std::path::PathBuf;

/// Process configuration.
pub struct Config {
    /// Socket address to bind, `JIZIFIN_BIND` (default `0.0.0.0:8000`).
    pub bind: String,
    /// DuckDB file, `JIZIFIN_DB_PATH` (default `/data/finance.duckdb`).
    pub db_path: PathBuf,
}

impl Config {
    /// Reads configuration, falling back to container defaults.
    pub fn from_env() -> Self {
        Self {
            bind: std::env::var("JIZIFIN_BIND").unwrap_or_else(|_| "0.0.0.0:8000".into()),
            db_path: std::env::var("JIZIFIN_DB_PATH").unwrap_or_else(|_| "/data/finance.duckdb".into()).into(),
        }
    }
}
