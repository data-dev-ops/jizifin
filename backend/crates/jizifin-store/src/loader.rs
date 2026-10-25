//! Loads Auditor snapshots through the repositories. Raw rows only: no contentious arithmetic in SQL.

use crate::repo::{Accounts, Agreements, Budgets, Categories, IncomeOverrides, Jobs, Members, OffsetRules, Projects};
use crate::rows::{self, ENTRY_COLS};
use crate::sql::query;
use crate::{Database, StoreError};
use duckdb::{Connection, params};
use jizifin_auditor::{AuditError, AuditScope, Snapshot, SnapshotLoader};
use jizifin_domain::entities::{Entry, Share};
use jizifin_domain::ids::EntryId;
use std::collections::BTreeMap;

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
            overrides: Self::overrides(conn, &entries)?,
            entries,
            jobs: Jobs::list(conn)?,
            income_overrides: IncomeOverrides::list(conn, scope.period)?,
            accounts: Accounts::all(conn)?,
            offset_rules: OffsetRules::active(conn)?,
            budgets: Budgets::list(conn, scope.period)?,
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

    fn overrides(conn: &Connection, entries: &[Entry]) -> Result<BTreeMap<EntryId, Vec<Share>>, StoreError> {
        let ids = duckdb::types::Value::List(entries.iter().map(|e| duckdb::types::Value::BigInt(e.id.0)).collect());
        let sql = "SELECT entry_id, member_id, pct FROM entry_share_overrides WHERE list_contains(?, entry_id) ORDER BY 1, 2";
        let mut map: BTreeMap<EntryId, Vec<Share>> = BTreeMap::new();
        query(conn, sql, [ids], rows::owned_share)?.into_iter().for_each(|(id, s)| map.entry(EntryId(id)).or_default().push(s));
        Ok(map)
    }
}
