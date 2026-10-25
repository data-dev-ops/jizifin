//! `jizifin-migrate --sqlite <legacy.db> --duckdb <new.duckdb>`
//!
//! Reads the legacy FastAPI SQLite database read-only and writes a fresh DuckDB database.
//! `JIZIFIN_PASSPHRASE` is used only to recompute the ciphertext of the category and member
//! names the legacy code hard-coded; it is never stored.

mod cipher;
mod error;
mod ids;
mod legacy;
mod report;
mod steps;

use error::MigrateError;
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("migration failed: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), MigrateError> {
    let (sqlite, duckdb) = paths()?;
    if duckdb.exists() {
        return Err(MigrateError::Usage(format!("{} already exists", duckdb.display())));
    }
    let passphrase = std::env::var("JIZIFIN_PASSPHRASE").unwrap_or_default();
    if passphrase.is_empty() {
        return Err(MigrateError::Usage("JIZIFIN_PASSPHRASE is not set".into()));
    }
    let report = steps::Migration::open(&sqlite, &duckdb, &passphrase)?.run()?;
    println!("{report}");
    Ok(())
}

fn paths() -> Result<(PathBuf, PathBuf), MigrateError> {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    match (value("--sqlite"), value("--duckdb")) {
        (Some(s), Some(d)) => Ok((s, d)),
        _ => Err(MigrateError::Usage("usage: jizifin-migrate --sqlite <legacy.db> --duckdb <new.duckdb>".into())),
    }
}
