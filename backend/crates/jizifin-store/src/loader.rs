//! Loads Auditor snapshots through the repositories. Raw rows only: no contentious arithmetic in SQL.

use crate::repo::{Accounts, Agreements, Budgets, Categories, Entries, IncomeOverrides, Jobs, Members, OffsetRules, Projects};
use crate::rows::{self, ENTRY_COLS};
use crate::sql::query;
use crate::{Database, StoreError};
use duckdb::{Connection, params};
use jizifin_auditor::{AuditError, AuditScope, Snapshot, SnapshotLoader};
use jizifin_domain::entities::Entry;

/// [`SnapshotLoader`] backed by DuckDB.
pub struct DuckSnapshotLoader {
    db: Database,
}

impl SnapshotLoader for DuckSnapshotLoader {
    fn load(&self, scope: &AuditScope) -> Result<Snapshot, AuditError> {
        self.db.read(|conn| Self::snapshot(conn, scope)).map_err(|e| AuditError::Load(e.to_string()))
    }
}

impl DuckSnapshotLoader {
    /// Loader reading from `db`.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    fn snapshot(conn: &Connection, scope: &AuditScope) -> Result<Snapshot, StoreError> {
        let entries = Self::entries(conn, scope)?;
        Ok(Snapshot {
            members: Members::list(conn)?,
            categories: Categories::map(conn)?,
            agreements: Agreements::all(conn)?,
            overrides: Entries::overrides_of(conn, entries.iter().map(|e| e.id))?,
            entries,
            jobs: Jobs::list(conn)?,
            income_overrides: IncomeOverrides::list(conn, scope.period)?,
            accounts: Accounts::all(conn)?,
            offset_rules: OffsetRules::active(conn)?,
            budgets: Budgets::all(conn)?,
            projects: Projects::all(conn)?,
        })
    }

    fn entries(conn: &Connection, scope: &AuditScope) -> Result<Vec<Entry>, StoreError> {
        let (from, to) = (scope.period.from, scope.period.to);
        let project_sql = "AND (project_id = ? OR kind = 'deposit')";
        let (filter, project) = match scope.project {
            Some(p) => (project_sql, Some(p.0)),
            None => ("AND COALESCE(?, 0) = 0", None),
        };
        let sql = format!("SELECT {ENTRY_COLS} FROM entries WHERE entry_date >= ? AND entry_date <= ? {filter} ORDER BY entry_date, id");
        query(conn, &sql, params![from, to, project], rows::entry)
    }
}
