use crate::rows::{self, SEAL_COLS};
use crate::sql::{exists, query};
use crate::StoreError;
use chrono::NaiveDate;
use duckdb::{Connection, params};
use jizifin_domain::Month;
use jizifin_domain::entities::MonthSeal;

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
