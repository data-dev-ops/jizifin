use super::Categories;
use crate::rows::{self, BUDGET_COLS};
use crate::sql::{query, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{Budget, CategoryKind};
use jizifin_domain::ids::CategoryId;
use jizifin_domain::{Month, Period};

/// Monthly category limits.
pub struct Budgets;

impl Budgets {
    /// Budgets inside `period`.
    pub fn list(conn: &Connection, period: Period) -> Result<Vec<Budget>, StoreError> {
        let sql = format!("SELECT {BUDGET_COLS} FROM budgets WHERE month >= date_trunc('month', ?::DATE) AND month <= ? ORDER BY month, category_id");
        query(conn, &sql, params![period.from, period.to], rows::budget)
    }

    /// Inserts or replaces the limit of `(category, month)`.
    pub fn upsert(conn: &Connection, input: &Budget) -> Result<(), StoreError> {
        if Categories::kind_of(conn, input.category)? != CategoryKind::Expense {
            return Err(StoreError::Invalid("budgets apply to expense categories only".into()));
        }
        let sql = "INSERT OR REPLACE INTO budgets (category_id, month, limit_cents, source) VALUES (?, ?, ?, ?)";
        conn.execute(sql, params![input.category.0, input.month.first_day(), input.limit.value(), input.source.label()])?;
        Ok(())
    }

    /// Deletes the limit of `(category, month)`.
    pub fn delete(conn: &Connection, category: CategoryId, month: Month) -> Result<(), StoreError> {
        let sql = "DELETE FROM budgets WHERE category_id = ? AND month = ?";
        touch_one(conn, sql, params![category.0, month.first_day()], "budget")
    }
}
