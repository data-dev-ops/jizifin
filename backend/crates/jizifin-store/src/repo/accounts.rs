use crate::inputs::AccountInput;
use crate::rows::{self, ACCOUNT_CATEGORY_COLS, ACCOUNT_COLS, ACCOUNT_MEMBER_COLS};
use crate::sql::{insert_id, query, require, touch_one, unreferenced};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{Account, AccountCategory, AccountMember};
use jizifin_domain::ids::AccountId;

/// Tables whose rows block deleting an account.
const REFERENCES: &[(&str, &str)] = &[("entries", "account_id"), ("recurring_templates", "account_id")];

/// Joint accounts with roster and categories.
pub struct Accounts;

impl Accounts {
    /// Every account, fully populated.
    pub fn all(conn: &Connection) -> Result<Vec<Account>, StoreError> {
        let mut accounts = query(conn, &format!("SELECT {ACCOUNT_COLS} FROM accounts ORDER BY id"), [], rows::account)?;
        let members = query(conn, &format!("SELECT {ACCOUNT_MEMBER_COLS} FROM account_members ORDER BY 1, 2"), [], rows::account_member)?;
        let categories = query(conn, &format!("SELECT {ACCOUNT_CATEGORY_COLS} FROM account_categories ORDER BY 1, 2"), [], rows::account_category)?;
        for account in &mut accounts {
            account.members = members.iter().filter(|(id, _)| *id == account.id).map(|(_, m)| m.clone()).collect();
            account.categories = categories.iter().filter(|(id, _)| *id == account.id).map(|(_, c)| c.clone()).collect();
        }
        Ok(accounts)
    }

    /// One account.
    pub fn get(conn: &Connection, id: AccountId) -> Result<Account, StoreError> {
        Self::all(conn)?.into_iter().find(|a| a.id == id).ok_or_else(|| StoreError::NotFound("account".into()))
    }

    /// Inserts an account with roster and categories.
    pub fn create(conn: &Connection, input: &AccountInput) -> Result<AccountId, StoreError> {
        Self::check(conn, input)?;
        let sql = "INSERT INTO accounts (name_ct, safety_margin_pct, deposit_mode) VALUES (?, ?, ?) RETURNING id";
        let id = AccountId(insert_id(conn, sql, params![input.name.as_str(), input.safety_margin_pct, input.deposit_mode.label()])?);
        Self::write_children(conn, id, input)?;
        Ok(id)
    }

    /// Replaces an account and its children.
    pub fn update(conn: &Connection, id: AccountId, input: &AccountInput) -> Result<(), StoreError> {
        Self::check(conn, input)?;
        let sql = "UPDATE accounts SET name_ct = ?, safety_margin_pct = ?, deposit_mode = ? WHERE id = ?";
        touch_one(conn, sql, params![input.name.as_str(), input.safety_margin_pct, input.deposit_mode.label(), id.0], "account")?;
        Self::write_children(conn, id, input)
    }

    /// Deletes an account no entry or template references.
    pub fn delete(conn: &Connection, id: AccountId) -> Result<(), StoreError> {
        unreferenced(conn, REFERENCES, id.0, "account")?;
        Self::clear_children(conn, id)?;
        touch_one(conn, "DELETE FROM accounts WHERE id = ?", [id.0], "account")
    }

    fn check(conn: &Connection, input: &AccountInput) -> Result<(), StoreError> {
        input.validate()?;
        input.members.iter().try_for_each(|m| require(conn, "members", m.member.0))?;
        input.categories.iter().try_for_each(|c| require(conn, "categories", c.category.0))
    }

    fn clear_children(conn: &Connection, id: AccountId) -> Result<(), StoreError> {
        conn.execute("DELETE FROM account_members WHERE account_id = ?", [id.0])?;
        conn.execute("DELETE FROM account_categories WHERE account_id = ?", [id.0])?;
        Ok(())
    }

    fn write_children(conn: &Connection, id: AccountId, input: &AccountInput) -> Result<(), StoreError> {
        Self::clear_children(conn, id)?;
        input.members.iter().try_for_each(|m| Self::insert_member(conn, id, m))?;
        input.categories.iter().try_for_each(|c| Self::insert_category(conn, id, c))
    }

    fn insert_member(conn: &Connection, id: AccountId, m: &AccountMember) -> Result<(), StoreError> {
        let sql = "INSERT INTO account_members (account_id, member_id, fixed_deposit_cents, due_day) VALUES (?, ?, ?, ?)";
        conn.execute(sql, params![id.0, m.member.0, m.fixed_deposit.map(|c| c.value()), m.due_day])?;
        Ok(())
    }

    fn insert_category(conn: &Connection, id: AccountId, c: &AccountCategory) -> Result<(), StoreError> {
        let sql = "INSERT INTO account_categories (account_id, category_id, expected_cents) VALUES (?, ?, ?)";
        conn.execute(sql, params![id.0, c.category.0, c.expected.value()])?;
        Ok(())
    }
}
