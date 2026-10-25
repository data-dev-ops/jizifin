use crate::inputs::CategoryInput;
use crate::rows::{self, CATEGORY_COLS};
use crate::sql::{insert_id, query, touch_one, unreferenced};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{Category, CategoryKind};
use jizifin_domain::ids::CategoryId;
use std::collections::BTreeMap;

/// Tables whose rows block deleting a category.
const REFERENCES: &[(&str, &str)] = &[
    ("entries", "category_id"),
    ("recurring_templates", "category_id"),
    ("account_categories", "category_id"),
    ("offset_rules", "credit_category_id"),
    ("offset_rules", "debit_category_id"),
];

/// Expense and income categories.
pub struct Categories;

impl Categories {
    /// Every category, ordered by id.
    pub fn list(conn: &Connection) -> Result<Vec<Category>, StoreError> {
        query(conn, &format!("SELECT {CATEGORY_COLS} FROM categories ORDER BY id"), [], rows::category)
    }

    /// Every category keyed by id.
    pub fn map(conn: &Connection) -> Result<BTreeMap<CategoryId, Category>, StoreError> {
        Ok(Self::list(conn)?.into_iter().map(|c| (c.id, c)).collect())
    }

    /// Kind of `id`, or `Invalid` when it does not exist.
    pub fn kind_of(conn: &Connection, id: CategoryId) -> Result<CategoryKind, StoreError> {
        Self::map(conn)?.get(&id).map(|c| c.kind).ok_or_else(|| StoreError::Invalid(format!("category {id} does not exist")))
    }

    /// Inserts a category.
    pub fn create(conn: &Connection, input: &CategoryInput) -> Result<CategoryId, StoreError> {
        let sql = "INSERT INTO categories (kind, name_ct, pay_mode) VALUES (?, ?, ?) RETURNING id";
        insert_id(conn, sql, params![input.kind.label(), input.name.as_str(), input.pay_mode.label()]).map(CategoryId)
    }

    /// Renames or re-modes a category. Kind is immutable once referenced.
    pub fn update(conn: &Connection, id: CategoryId, input: &CategoryInput) -> Result<(), StoreError> {
        if Self::kind_of(conn, id)? != input.kind {
            unreferenced(conn, REFERENCES, id.0, "category")?;
        }
        let sql = "UPDATE categories SET kind = ?, name_ct = ?, pay_mode = ? WHERE id = ?";
        touch_one(conn, sql, params![input.kind.label(), input.name.as_str(), input.pay_mode.label(), id.0], "category")
    }

    /// Deletes an unreferenced category with its agreements and budgets.
    pub fn delete(conn: &Connection, id: CategoryId) -> Result<(), StoreError> {
        unreferenced(conn, REFERENCES, id.0, "category")?;
        conn.execute("DELETE FROM agreement_shares WHERE agreement_id IN (SELECT id FROM agreements WHERE category_id = ?)", [id.0])?;
        conn.execute("DELETE FROM agreements WHERE category_id = ?", [id.0])?;
        conn.execute("DELETE FROM budgets WHERE category_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM categories WHERE id = ?", [id.0], "category")
    }
}
