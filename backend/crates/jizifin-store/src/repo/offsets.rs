use crate::inputs::OffsetRuleInput;
use crate::rows::{self, OFFSET_RULE_COLS};
use crate::sql::{insert_id, query, require, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::OffsetRule;
use jizifin_domain::ids::OffsetRuleId;

/// Settlement netting rules.
pub struct OffsetRules;

impl OffsetRules {
    /// Every rule.
    pub fn list(conn: &Connection) -> Result<Vec<OffsetRule>, StoreError> {
        query(conn, &format!("SELECT {OFFSET_RULE_COLS} FROM offset_rules ORDER BY id"), [], rows::offset_rule)
    }

    /// Active rules only.
    pub fn active(conn: &Connection) -> Result<Vec<OffsetRule>, StoreError> {
        Ok(Self::list(conn)?.into_iter().filter(|r| r.is_active).collect())
    }

    /// Inserts a rule.
    pub fn create(conn: &Connection, input: &OffsetRuleInput) -> Result<OffsetRuleId, StoreError> {
        Self::check(conn, input)?;
        let sql = "INSERT INTO offset_rules (credit_member_id, credit_category_id, debit_member_id, debit_category_id, is_active) \
                   VALUES (?, ?, ?, ?, ?) RETURNING id";
        let p = params![input.credit_member.0, input.credit_category.0, input.debit_member.0, input.debit_category.0, input.is_active];
        insert_id(conn, sql, p).map(OffsetRuleId)
    }

    /// Deletes a rule.
    pub fn delete(conn: &Connection, id: OffsetRuleId) -> Result<(), StoreError> {
        touch_one(conn, "DELETE FROM offset_rules WHERE id = ?", [id.0], "offset rule")
    }

    fn check(conn: &Connection, input: &OffsetRuleInput) -> Result<(), StoreError> {
        require(conn, "members", input.credit_member.0)?;
        require(conn, "members", input.debit_member.0)?;
        require(conn, "categories", input.credit_category.0)?;
        require(conn, "categories", input.debit_category.0)
    }
}
