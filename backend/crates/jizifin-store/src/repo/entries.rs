use super::{Categories, Seals, Tags};
use crate::inputs::{DuplicateProbe, EntryInput};
use crate::rows::{self, ENTRY_COLS};
use crate::sql::{DynParams, insert_id, opt_ct, query, require, touch_one};
use crate::StoreError;
use chrono::NaiveDate;
use duckdb::{Connection, params, params_from_iter};
use jizifin_domain::entities::{CategoryKind, Entry, EntryKind, Share};
use jizifin_domain::ids::{AccountId, CategoryId, EntryId, MemberId, ProjectId, TagId};
use jizifin_domain::Month;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default and maximum page size.
const PAGE_SIZE: usize = 200;

/// Query-string filter for listing entries. All fields optional.
#[derive(Debug, Default, Deserialize)]
pub struct EntryFilter {
    /// First date included.
    pub from: Option<NaiveDate>,
    /// Last date included.
    pub to: Option<NaiveDate>,
    /// Entry type.
    pub kind: Option<EntryKind>,
    /// Member.
    pub member: Option<MemberId>,
    /// Category.
    pub category: Option<CategoryId>,
    /// Account.
    pub account: Option<AccountId>,
    /// Project.
    pub project: Option<ProjectId>,
    /// Tag.
    pub tag: Option<TagId>,
    /// Opaque cursor from a previous page (`YYYY-MM-DD:id`).
    pub cursor: Option<String>,
    /// Page size, capped at 200.
    pub limit: Option<usize>,
}

/// An entry with its custom split.
#[derive(Debug, Serialize)]
pub struct EntryView {
    /// The entry.
    #[serde(flatten)]
    pub entry: Entry,
    /// Custom split; empty when none.
    pub overrides: Vec<Share>,
}

/// One page of entries, newest first.
#[derive(Debug, Serialize)]
pub struct EntryPage {
    /// Entries.
    pub entries: Vec<EntryView>,
    /// Cursor for the next page, absent on the last page.
    pub next_cursor: Option<String>,
}

/// Existing entries matching one probe.
#[derive(Debug, Serialize)]
pub struct DuplicateMatch {
    /// Index of the probe in the request.
    pub index: usize,
    /// Matching entries.
    pub matches: Vec<Entry>,
}

/// The unified ledger.
pub struct Entries;

impl Entries {
    /// Filtered, keyset-paginated listing.
    pub fn list(conn: &Connection, filter: &EntryFilter) -> Result<EntryPage, StoreError> {
        let limit = filter.limit.unwrap_or(PAGE_SIZE).clamp(1, PAGE_SIZE);
        let (sql, params) = Self::list_sql(filter, limit)?;
        let mut found = query(conn, &sql, params_from_iter(params), rows::entry)?;
        let next_cursor = (found.len() > limit).then(|| Self::cursor_of(&found[limit - 1]));
        found.truncate(limit);
        Ok(EntryPage { entries: Self::with_overrides(conn, found)?, next_cursor })
    }

    /// One entry.
    pub fn get(conn: &Connection, id: EntryId) -> Result<Entry, StoreError> {
        let found = query(conn, &format!("SELECT {ENTRY_COLS} FROM entries WHERE id = ?"), [id.0], rows::entry)?;
        found.into_iter().next().ok_or_else(|| StoreError::NotFound("entry".into()))
    }

    /// Inserts an entry with its custom split.
    pub fn create(conn: &Connection, input: &EntryInput) -> Result<EntryId, StoreError> {
        Self::check(conn, input)?;
        let sql = format!("INSERT INTO entries ({}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id", Self::WRITE_COLS);
        let id = EntryId(insert_id(conn, &sql, params_from_iter(Self::params(input)))?);
        Self::write_overrides(conn, id, &input.overrides)?;
        Ok(id)
    }

    /// Inserts many entries; all or nothing (the caller's transaction).
    pub fn create_batch(conn: &Connection, inputs: &[EntryInput]) -> Result<Vec<EntryId>, StoreError> {
        inputs.iter().map(|input| Self::create(conn, input)).collect()
    }

    /// Replaces an entry; both the old and the new month must be unsealed.
    pub fn update(conn: &Connection, id: EntryId, input: &EntryInput) -> Result<(), StoreError> {
        Seals::guard(conn, &[Self::get(conn, id)?.date])?;
        Self::check(conn, input)?;
        let assignments = Self::WRITE_COLS.split(", ").map(|c| format!("{c} = ?")).collect::<Vec<_>>().join(", ");
        let mut p = Self::params(input);
        p.push(Box::new(id.0));
        touch_one(conn, &format!("UPDATE entries SET {assignments} WHERE id = ?"), params_from_iter(p), "entry")?;
        Self::write_overrides(conn, id, &input.overrides)
    }

    /// Deletes an entry in an unsealed month.
    pub fn delete(conn: &Connection, id: EntryId) -> Result<(), StoreError> {
        Seals::guard(conn, &[Self::get(conn, id)?.date])?;
        conn.execute("DELETE FROM entry_share_overrides WHERE entry_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM entries WHERE id = ?", [id.0], "entry")
    }

    /// Same category and amount in the same month, per probe.
    pub fn duplicates(conn: &Connection, probes: &[DuplicateProbe]) -> Result<Vec<DuplicateMatch>, StoreError> {
        let matches = probes.iter().enumerate().map(|(index, p)| Ok(DuplicateMatch { index, matches: Self::probe(conn, p)? }));
        let all = matches.collect::<Result<Vec<_>, StoreError>>()?;
        Ok(all.into_iter().filter(|m| !m.matches.is_empty()).collect())
    }

    const WRITE_COLS: &'static str =
        "kind, entry_date, amount_cents, member_id, category_id, account_id, project_id, tag_id, name_ct, note_ct";

    fn params(input: &EntryInput) -> DynParams {
        vec![
            Box::new(input.kind.label()),
            Box::new(input.date),
            Box::new(input.amount.value()),
            Box::new(input.member.map(|m| m.0)),
            Box::new(input.category.map(|c| c.0)),
            Box::new(input.account.map(|a| a.0)),
            Box::new(input.project.map(|p| p.0)),
            Box::new(input.tag.map(|t| t.0)),
            Box::new(opt_ct(&input.name).map(str::to_owned)),
            Box::new(opt_ct(&input.note).map(str::to_owned)),
        ]
    }

    fn check(conn: &Connection, input: &EntryInput) -> Result<(), StoreError> {
        input.validate()?;
        Seals::guard(conn, &[input.date])?;
        Self::check_references(conn, input)?;
        Self::check_category_kind(conn, input)?;
        input.tag.map_or(Ok(()), |tag| Self::check_tag_window(conn, tag, input.date))
    }

    fn check_references(conn: &Connection, input: &EntryInput) -> Result<(), StoreError> {
        let refs = [
            ("members", input.member.map(|m| m.0)),
            ("accounts", input.account.map(|a| a.0)),
            ("projects", input.project.map(|p| p.0)),
        ];
        refs.iter().filter_map(|(t, id)| id.map(|id| (*t, id))).try_for_each(|(t, id)| require(conn, t, id))?;
        input.overrides.iter().try_for_each(|s| require(conn, "members", s.member.0))
    }

    fn check_category_kind(conn: &Connection, input: &EntryInput) -> Result<(), StoreError> {
        let Some(category) = input.category else { return Ok(()) };
        let expected = match input.kind {
            EntryKind::Income => CategoryKind::Income,
            _ => CategoryKind::Expense,
        };
        match Categories::kind_of(conn, category)? == expected {
            true => Ok(()),
            false => Err(StoreError::Invalid(format!("category {category} is not a {} category", expected.label()))),
        }
    }

    fn check_tag_window(conn: &Connection, tag: TagId, date: NaiveDate) -> Result<(), StoreError> {
        match Tags::get(conn, tag)?.allows(date) {
            true => Ok(()),
            false => Err(StoreError::Invalid(format!("{date} is outside the active window of tag {tag}"))),
        }
    }

    fn write_overrides(conn: &Connection, id: EntryId, shares: &[Share]) -> Result<(), StoreError> {
        conn.execute("DELETE FROM entry_share_overrides WHERE entry_id = ?", [id.0])?;
        let mut stmt = conn.prepare_cached("INSERT INTO entry_share_overrides (entry_id, member_id, pct) VALUES (?, ?, ?)")?;
        shares.iter().try_for_each(|s| stmt.execute(params![id.0, s.member.0, s.pct]).map(|_| ()))?;
        Ok(())
    }

    fn with_overrides(conn: &Connection, entries: Vec<Entry>) -> Result<Vec<EntryView>, StoreError> {
        let ids: Vec<i64> = entries.iter().map(|e| e.id.0).collect();
        let mut by_entry: BTreeMap<i64, Vec<Share>> = BTreeMap::new();
        let sql = "SELECT entry_id, member_id, pct FROM entry_share_overrides WHERE list_contains(?, entry_id) ORDER BY 1, 2";
        let list = duckdb::types::Value::List(ids.into_iter().map(duckdb::types::Value::BigInt).collect());
        query(conn, sql, [list], rows::owned_share)?.into_iter().for_each(|(id, s)| by_entry.entry(id).or_default().push(s));
        Ok(entries.into_iter().map(|e| EntryView { overrides: by_entry.remove(&e.id.0).unwrap_or_default(), entry: e }).collect())
    }

    fn probe(conn: &Connection, p: &DuplicateProbe) -> Result<Vec<Entry>, StoreError> {
        let period = Month::of(p.date).period();
        let sql = format!(
            "SELECT {ENTRY_COLS} FROM entries WHERE category_id = ? AND amount_cents = ? AND entry_date >= ? AND entry_date <= ? \
             AND id <> COALESCE(?, -1) ORDER BY entry_date, id"
        );
        query(conn, &sql, params![p.category.0, p.amount.value(), period.from, period.to, p.exclude.map(|e| e.0)], rows::entry)
    }

    fn cursor_of(entry: &Entry) -> String {
        format!("{}:{}", entry.date, entry.id)
    }

    fn parse_cursor(raw: &str) -> Result<(NaiveDate, i64), StoreError> {
        let invalid = || StoreError::Invalid(format!("invalid cursor '{raw}'"));
        let (date, id) = raw.split_once(':').ok_or_else(invalid)?;
        Ok((date.parse().map_err(|_| invalid())?, id.parse().map_err(|_| invalid())?))
    }

    fn list_sql(filter: &EntryFilter, limit: usize) -> Result<(String, DynParams), StoreError> {
        let mut clauses: Vec<&str> = Vec::new();
        let mut params: DynParams = Vec::new();
        Self::push_dates(filter, &mut clauses, &mut params);
        Self::push_ids(filter, &mut clauses, &mut params);
        Self::push_cursor(filter, &mut clauses, &mut params)?;
        let filter_sql = match clauses.is_empty() {
            true => String::new(),
            false => format!("WHERE {}", clauses.join(" AND ")),
        };
        let sql = format!("SELECT {ENTRY_COLS} FROM entries {filter_sql} ORDER BY entry_date DESC, id DESC LIMIT {}", limit + 1);
        Ok((sql, params))
    }

    fn push_dates(filter: &EntryFilter, clauses: &mut Vec<&str>, params: &mut DynParams) {
        if let Some(from) = filter.from {
            clauses.push("entry_date >= ?");
            params.push(Box::new(from));
        }
        if let Some(to) = filter.to {
            clauses.push("entry_date <= ?");
            params.push(Box::new(to));
        }
        if let Some(kind) = filter.kind {
            clauses.push("kind = ?");
            params.push(Box::new(kind.label()));
        }
    }

    fn push_ids(filter: &EntryFilter, clauses: &mut Vec<&str>, params: &mut DynParams) {
        let ids = [
            ("member_id = ?", filter.member.map(|v| v.0)),
            ("category_id = ?", filter.category.map(|v| v.0)),
            ("account_id = ?", filter.account.map(|v| v.0)),
            ("project_id = ?", filter.project.map(|v| v.0)),
            ("tag_id = ?", filter.tag.map(|v| v.0)),
        ];
        for (clause, value) in ids.into_iter().filter_map(|(c, v)| v.map(|v| (c, v))) {
            clauses.push(clause);
            params.push(Box::new(value));
        }
    }

    fn push_cursor(filter: &EntryFilter, clauses: &mut Vec<&str>, params: &mut DynParams) -> Result<(), StoreError> {
        let Some(raw) = &filter.cursor else { return Ok(()) };
        let (date, id) = Self::parse_cursor(raw)?;
        clauses.push("(entry_date < ? OR (entry_date = ? AND id < ?))");
        params.extend([Box::new(date) as Box<dyn duckdb::ToSql>, Box::new(date), Box::new(id)]);
        Ok(())
    }
}
