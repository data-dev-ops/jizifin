//! Loads Auditor snapshots. Raw rows only: no contentious arithmetic happens in SQL.

mod rows;

use crate::{Database, StoreError};
use duckdb::{Connection, Params, Row};
use jizifin_auditor::{AuditError, AuditScope, Snapshot, SnapshotLoader};
use jizifin_domain::entities::{Account, Agreement};
use jizifin_domain::ids::{AgreementId, CategoryId, EntryId};
use std::collections::BTreeMap;

/// [`SnapshotLoader`] backed by DuckDB.
pub struct DuckSnapshotLoader {
    db: Database,
}

impl SnapshotLoader for DuckSnapshotLoader {
    fn load(&self, scope: &AuditScope) -> Result<Snapshot, AuditError> {
        self.load_snapshot(scope).map_err(|e| AuditError::Load(e.to_string()))
    }
}

impl DuckSnapshotLoader {
    /// Loader reading from `db`.
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    fn load_snapshot(&self, scope: &AuditScope) -> Result<Snapshot, StoreError> {
        let conn = self.db.connection()?;
        let (from, to) = (scope.period.from, scope.period.to);
        Ok(Snapshot {
            members: query(&conn, "SELECT id, name_ct, color, is_active FROM members ORDER BY id", [], rows::member)?,
            categories: Self::categories(&conn)?,
            agreements: Self::agreements(&conn)?,
            entries: query(&conn, ENTRIES_SQL, [from, to], rows::entry)?,
            overrides: Self::overrides(&conn, scope)?,
            jobs: query(&conn, JOBS_SQL, [], rows::job)?,
            income_overrides: query(&conn, INCOME_OVERRIDES_SQL, [from, to], rows::income_override)?,
            accounts: Self::accounts(&conn)?,
            offset_rules: query(&conn, OFFSET_RULES_SQL, [], rows::offset_rule)?,
            budgets: query(&conn, BUDGETS_SQL, [from, to], rows::budget)?,
        })
    }

    fn categories(conn: &Connection) -> Result<BTreeMap<CategoryId, jizifin_domain::entities::Category>, StoreError> {
        let list = query(conn, "SELECT id, kind, name_ct, pay_mode FROM categories ORDER BY id", [], rows::category)?;
        Ok(list.into_iter().map(|c| (c.id, c)).collect())
    }

    fn agreements(conn: &Connection) -> Result<Vec<Agreement>, StoreError> {
        let mut agreements = query(conn, AGREEMENTS_SQL, [], |r| {
            Ok(Agreement { id: AgreementId(r.get(0)?), category: CategoryId(r.get(1)?), valid_from: r.get(2)?, valid_to: r.get(3)?, shares: Vec::new() })
        })?;
        let shares = query(conn, AGREEMENT_SHARES_SQL, [], |r| Ok((AgreementId(r.get(0)?), rows::share(r, 1)?)))?;
        for (id, share) in shares {
            agreements.iter_mut().filter(|a| a.id == id).for_each(|a| a.shares.push(share));
        }
        Ok(agreements)
    }

    fn overrides(conn: &Connection, scope: &AuditScope) -> Result<BTreeMap<EntryId, Vec<jizifin_domain::entities::Share>>, StoreError> {
        let list = query(conn, OVERRIDES_SQL, [scope.period.from, scope.period.to], |r| Ok((EntryId(r.get(0)?), rows::share(r, 1)?)))?;
        let mut map: BTreeMap<EntryId, Vec<_>> = BTreeMap::new();
        list.into_iter().for_each(|(id, share)| map.entry(id).or_default().push(share));
        Ok(map)
    }

    fn accounts(conn: &Connection) -> Result<Vec<Account>, StoreError> {
        let mut accounts = query(conn, "SELECT id, safety_margin_pct, deposit_mode FROM accounts ORDER BY id", [], rows::account)?;
        for (id, member) in query(conn, ACCOUNT_MEMBERS_SQL, [], rows::account_member)? {
            accounts.iter_mut().filter(|a| a.id == id).for_each(|a| a.members.push(member.clone()));
        }
        for (id, category) in query(conn, ACCOUNT_CATEGORIES_SQL, [], rows::account_category)? {
            accounts.iter_mut().filter(|a| a.id == id).for_each(|a| a.categories.push(category.clone()));
        }
        Ok(accounts)
    }
}

fn query<T, P: Params>(conn: &Connection, sql: &str, params: P, map: impl FnMut(&Row<'_>) -> duckdb::Result<T>) -> Result<Vec<T>, StoreError> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_map(params, map)?;
    Ok(rows.collect::<duckdb::Result<Vec<T>>>()?)
}

const ENTRIES_SQL: &str = "SELECT id, kind, entry_date, amount_cents, member_id, category_id, account_id, project_id, tag_id \
    FROM entries WHERE entry_date >= ? AND entry_date <= ? ORDER BY entry_date, id";
const JOBS_SQL: &str = "SELECT id, member_id, amount_cents, frequency, valid_from, valid_to FROM jobs ORDER BY id";
const INCOME_OVERRIDES_SQL: &str =
    "SELECT member_id, month, amount_cents FROM income_overrides WHERE month >= date_trunc('month', ?::DATE) AND month <= ? ORDER BY member_id, month";
const AGREEMENTS_SQL: &str = "SELECT id, category_id, valid_from, valid_to FROM agreements ORDER BY id";
const AGREEMENT_SHARES_SQL: &str = "SELECT agreement_id, member_id, pct FROM agreement_shares ORDER BY agreement_id, member_id";
const OVERRIDES_SQL: &str = "SELECT o.entry_id, o.member_id, o.pct FROM entry_share_overrides o \
    JOIN entries e ON e.id = o.entry_id WHERE e.entry_date >= ? AND e.entry_date <= ? ORDER BY o.entry_id, o.member_id";
const ACCOUNT_MEMBERS_SQL: &str = "SELECT account_id, member_id, fixed_deposit_cents, due_day FROM account_members ORDER BY account_id, member_id";
const ACCOUNT_CATEGORIES_SQL: &str = "SELECT account_id, category_id, expected_cents FROM account_categories ORDER BY account_id, category_id";
const OFFSET_RULES_SQL: &str = "SELECT id, credit_member_id, credit_category_id, debit_member_id, debit_category_id \
    FROM offset_rules WHERE is_active ORDER BY id";
const BUDGETS_SQL: &str =
    "SELECT category_id, month, limit_cents, source FROM budgets WHERE month >= date_trunc('month', ?::DATE) AND month <= ? ORDER BY category_id, month";
