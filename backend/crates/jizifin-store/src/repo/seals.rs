use crate::rows::{self, SEAL_COLS};
use crate::sql::{exists, query, touch_one};
use crate::StoreError;
use chrono::NaiveDate;
use duckdb::{Connection, params};
use jizifin_domain::Month;
use jizifin_domain::entities::MonthSeal;

/// Validity window of master data such as a job or an agreement; `to == None` is open-ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// First day.
    pub from: NaiveDate,
    /// Last day, or `None` while open-ended.
    pub to: Option<NaiveDate>,
}

impl Span {
    /// Smallest window covering both.
    fn union(self, other: Self) -> Self {
        let to = self.to.zip(other.to).map(|(a, b)| a.max(b));
        Self { from: self.from.min(other.from), to }
    }

    /// Days between the two end dates: the only part a pure end-date change touches.
    fn tail_change(self, other: Self) -> Self {
        let ends = self.to.zip(other.to);
        let from = ends.map_or_else(|| self.to.or(other.to).unwrap_or(self.from), |(a, b)| a.min(b));
        Self { from, to: ends.map(|(a, b)| a.max(b)) }
    }
}

/// Sealed month settlements. A sealed month accepts no ledger writes.
pub struct Seals;

impl Seals {
    /// Every seal, oldest first.
    pub fn list(conn: &Connection) -> Result<Vec<MonthSeal>, StoreError> {
        query(conn, &format!("SELECT {SEAL_COLS} FROM month_seals ORDER BY month"), [], rows::seal)
    }

    /// Seal of `month`, if any.
    pub fn get(conn: &Connection, month: Month) -> Result<Option<MonthSeal>, StoreError> {
        let found = query(conn, &format!("SELECT {SEAL_COLS} FROM month_seals WHERE month = ?"), [month.first_day()], rows::seal)?;
        Ok(found.into_iter().next())
    }

    /// Stores a seal; a month can be sealed once.
    pub fn insert(conn: &Connection, month: Month, result_json: &str, inputs_hash: &str) -> Result<(), StoreError> {
        if Self::get(conn, month)?.is_some() {
            return Err(StoreError::Conflict(format!("{month} is already sealed")));
        }
        let sql = "INSERT INTO month_seals (month, result_json, inputs_hash) VALUES (?, ?::JSON, ?)";
        conn.execute(sql, params![month.first_day(), result_json, inputs_hash])?;
        Ok(())
    }

    /// Deletes the seal of `month`, reopening it for ledger writes.
    pub fn delete(conn: &Connection, month: Month) -> Result<(), StoreError> {
        touch_one(conn, "DELETE FROM month_seals WHERE month = ?", [month.first_day()], "seal")
    }

    /// Fails with `Conflict` when any month touched by `span` is sealed.
    pub fn guard_span(conn: &Connection, span: Span) -> Result<(), StoreError> {
        let first = Month::of(span.from).first_day();
        let sql = "SELECT 1 FROM month_seals WHERE month >= ? AND (?::DATE IS NULL OR month <= ?) LIMIT 1";
        match exists(conn, sql, params![first, span.to, span.to])? {
            true => Err(StoreError::Conflict(format!("{} to {} touches a sealed month", span.from, span.to.map_or("open end".into(), |d| d.to_string())))),
            false => Ok(()),
        }
    }

    /// Guards replacing `old` with `new`. A change of content touches both windows; a pure end-date change only
    /// touches the days between the two end dates.
    pub fn guard_update(conn: &Connection, old: Span, new: Span, content_changed: bool) -> Result<(), StoreError> {
        let touched = match content_changed || old.from != new.from {
            true => old.union(new),
            false => old.tail_change(new),
        };
        match old == new && !content_changed {
            true => Ok(()),
            false => Self::guard_span(conn, touched),
        }
    }

    /// Fails with `Conflict` when any of `dates` lies in a sealed month.
    pub fn guard(conn: &Connection, dates: &[NaiveDate]) -> Result<(), StoreError> {
        for date in dates {
            let month = Month::of(*date);
            if exists(conn, "SELECT 1 FROM month_seals WHERE month = ? LIMIT 1", [month.first_day()])? {
                return Err(StoreError::Conflict(format!("{month} is sealed")));
            }
        }
        Ok(())
    }
}
