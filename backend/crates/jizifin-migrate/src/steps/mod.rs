//! Ordered migration steps, all inside one DuckDB transaction.

mod accounts;
mod catalog;
mod ledger;
mod people;
mod seal;
mod verify;

use crate::cipher::Markers;
use crate::error::MigrateError;
use crate::ids::IdMap;
use crate::legacy::Legacy;
use crate::report::Report;
use chrono::NaiveDate;
use jizifin_domain::{Ciphertext, Month};
use jizifin_store::{Connection, Database};
use std::path::Path;

/// Owns both databases for the duration of the migration.
pub struct Migration {
    legacy: Legacy,
    db: Database,
    markers: Markers,
}

impl Migration {
    /// Opens the legacy file read-only and creates the new database.
    pub fn open(sqlite: &Path, duckdb: &Path, passphrase: &str) -> Result<Self, MigrateError> {
        Ok(Self { legacy: Legacy::open(sqlite)?, db: Database::open(duckdb)?, markers: Markers::derive(passphrase) })
    }

    /// Copies everything in one transaction, verifies totals, then reseals settled months.
    pub fn run(self) -> Result<Report, MigrateError> {
        let mut report = Report::default();
        let mut conn = self.db.connection()?;
        let tx = conn.transaction().map_err(jizifin_store::StoreError::from)?;
        Steps { conn: &tx, legacy: &self.legacy, markers: &self.markers, ids: IdMap::default(), report: &mut report }.apply()?;
        tx.commit().map_err(jizifin_store::StoreError::from)?;
        seal::reseal(&self.db, &self.legacy.settled_months()?, &mut report)?;
        Ok(report)
    }
}

/// Shared context of every step.
pub(crate) struct Steps<'a> {
    conn: &'a Connection,
    legacy: &'a Legacy,
    markers: &'a Markers,
    ids: IdMap,
    report: &'a mut Report,
}

impl Steps<'_> {
    fn apply(mut self) -> Result<(), MigrateError> {
        self.members()?;
        self.categories()?;
        self.agreements()?;
        self.accounts()?;
        self.projects()?;
        self.tags()?;
        self.ledger()?;
        self.income_streams()?;
        self.recurring()?;
        self.budgets()?;
        self.offset_rule()?;
        self.auth()?;
        self.verify()
    }
}

/// Parses a legacy `YYYY-MM-DD` date.
pub(crate) fn date(raw: &str) -> Result<NaiveDate, MigrateError> {
    raw.parse().map_err(|_| MigrateError::Data(format!("invalid date '{raw}'")))
}

/// Parses an optional legacy date.
pub(crate) fn opt_date(raw: Option<&str>) -> Result<Option<NaiveDate>, MigrateError> {
    raw.filter(|r| !r.is_empty()).map(date).transpose()
}

/// Parses a legacy `YYYY-MM` month.
pub(crate) fn month(raw: &str) -> Result<Month, MigrateError> {
    Ok(Month::parse(raw)?)
}

/// Wraps legacy ciphertext.
pub(crate) fn ct(raw: &str) -> Result<Ciphertext, MigrateError> {
    Ok(Ciphertext::parse(raw)?)
}

/// Wraps optional legacy ciphertext; empty strings become `None`.
pub(crate) fn opt_ct(raw: Option<&str>) -> Result<Option<Ciphertext>, MigrateError> {
    raw.filter(|r| !r.is_empty()).map(ct).transpose()
}
