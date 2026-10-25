use super::Categories;
use crate::inputs::BudgetInput;
use crate::rows::{self, BUDGET_COLS};
use crate::sql::{insert_id, query, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{Budget, CategoryKind};
use jizifin_domain::ids::BudgetId;

/// SCD2 category limits: standing rows plus month-range overrides.
pub struct Budgets;

impl Budgets {
    /// Every budget row.
    pub fn all(conn: &Connection) -> Result<Vec<Budget>, StoreError> {
        query(conn, &format!("SELECT {BUDGET_COLS} FROM budgets ORDER BY category_id, valid_from, id"), [], rows::budget)
    }

    /// Inserts a budget row.
    pub fn create(conn: &Connection, input: &BudgetInput) -> Result<BudgetId, StoreError> {
        Self::check(conn, input)?;
        let sql = "INSERT INTO budgets (category_id, valid_from, valid_to, limit_cents, source) VALUES (?, ?, ?, ?, ?) RETURNING id";
        insert_id(conn, sql, Self::params(input)).map(BudgetId)
    }

    /// Replaces a budget row.
    pub fn update(conn: &Connection, id: BudgetId, input: &BudgetInput) -> Result<(), StoreError> {
        Self::check(conn, input)?;
        let sql = "UPDATE budgets SET category_id = ?, valid_from = ?, valid_to = ?, limit_cents = ?, source = ? WHERE id = ?";
        let (c, from, to, limit, source) = Self::params(input);
        touch_one(conn, sql, params![c, from, to, limit, source, id.0], "budget")
    }

    /// Deletes a budget row.
    pub fn delete(conn: &Connection, id: BudgetId) -> Result<(), StoreError> {
        touch_one(conn, "DELETE FROM budgets WHERE id = ?", [id.0], "budget")
    }

    fn check(conn: &Connection, input: &BudgetInput) -> Result<(), StoreError> {
        input.validate()?;
        match Categories::kind_of(conn, input.category)? {
            CategoryKind::Expense => Ok(()),
            CategoryKind::Income => Err(StoreError::Invalid("budgets apply to expense categories only".into())),
        }
    }

    fn params(input: &BudgetInput) -> (i64, chrono::NaiveDate, Option<chrono::NaiveDate>, i64, &'static str) {
        let to = input.valid_to.map(|m| m.first_day());
        (input.category.0, input.valid_from.first_day(), to, input.limit.value(), input.source.label())
    }
}
