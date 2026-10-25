use super::Categories;
use crate::inputs::AgreementInput;
use crate::rows::{self, AGREEMENT_COLS};
use crate::sql::{insert_id, opt_ct, query, require, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{Agreement, CategoryKind, Share};
use jizifin_domain::ids::{AgreementId, CategoryId};

/// SCD2 split agreements with their member shares.
pub struct Agreements;

impl Agreements {
    /// Every agreement with shares.
    pub fn all(conn: &Connection) -> Result<Vec<Agreement>, StoreError> {
        let mut agreements = query(conn, &format!("SELECT {AGREEMENT_COLS} FROM agreements ORDER BY id"), [], rows::agreement)?;
        let shares = query(conn, "SELECT agreement_id, member_id, pct FROM agreement_shares ORDER BY agreement_id, member_id", [], rows::owned_share)?;
        for (id, share) in shares {
            agreements.iter_mut().filter(|a| a.id.0 == id).for_each(|a| a.shares.push(share));
        }
        Ok(agreements)
    }

    /// Agreements of one category, newest start first.
    pub fn for_category(conn: &Connection, category: CategoryId) -> Result<Vec<Agreement>, StoreError> {
        let mut list: Vec<Agreement> = Self::all(conn)?.into_iter().filter(|a| a.category == category).collect();
        list.sort_by(|a, b| b.valid_from.cmp(&a.valid_from).then(b.id.cmp(&a.id)));
        Ok(list)
    }

    /// Inserts an agreement for an expense category.
    pub fn create(conn: &Connection, category: CategoryId, input: &AgreementInput) -> Result<AgreementId, StoreError> {
        Self::check(conn, category, input)?;
        let sql = "INSERT INTO agreements (category_id, valid_from, valid_to, note_ct) VALUES (?, ?, ?, ?) RETURNING id";
        let id = AgreementId(insert_id(conn, sql, params![category.0, input.valid_from, input.valid_to, opt_ct(&input.note)])?);
        Self::write_shares(conn, id, &input.shares)?;
        Ok(id)
    }

    /// Replaces an agreement's window, note and shares.
    pub fn update(conn: &Connection, id: AgreementId, input: &AgreementInput) -> Result<(), StoreError> {
        let category = Self::category_of(conn, id)?;
        Self::check(conn, category, input)?;
        let sql = "UPDATE agreements SET valid_from = ?, valid_to = ?, note_ct = ? WHERE id = ?";
        touch_one(conn, sql, params![input.valid_from, input.valid_to, opt_ct(&input.note), id.0], "agreement")?;
        Self::write_shares(conn, id, &input.shares)
    }

    /// Deletes an agreement and its shares.
    pub fn delete(conn: &Connection, id: AgreementId) -> Result<(), StoreError> {
        conn.execute("DELETE FROM agreement_shares WHERE agreement_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM agreements WHERE id = ?", [id.0], "agreement")
    }

    fn check(conn: &Connection, category: CategoryId, input: &AgreementInput) -> Result<(), StoreError> {
        input.validate()?;
        input.shares.iter().try_for_each(|s| require(conn, "members", s.member.0))?;
        match Categories::kind_of(conn, category)? {
            CategoryKind::Expense => Ok(()),
            CategoryKind::Income => Err(StoreError::Invalid("agreements apply to expense categories only".into())),
        }
    }

    fn category_of(conn: &Connection, id: AgreementId) -> Result<CategoryId, StoreError> {
        let found = query(conn, "SELECT category_id FROM agreements WHERE id = ?", [id.0], |r| r.get(0))?;
        found.first().copied().map(CategoryId).ok_or_else(|| StoreError::NotFound("agreement".into()))
    }

    fn write_shares(conn: &Connection, id: AgreementId, shares: &[Share]) -> Result<(), StoreError> {
        conn.execute("DELETE FROM agreement_shares WHERE agreement_id = ?", [id.0])?;
        let mut stmt = conn.prepare_cached("INSERT INTO agreement_shares (agreement_id, member_id, pct) VALUES (?, ?, ?)")?;
        shares.iter().try_for_each(|s| stmt.execute(params![id.0, s.member.0, s.pct]).map(|_| ()))?;
        Ok(())
    }
}
