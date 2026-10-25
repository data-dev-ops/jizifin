use crate::inputs::MemberInput;
use crate::rows::{self, MEMBER_COLS};
use crate::sql::{insert_id, query, touch_one, unreferenced};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::Member;
use jizifin_domain::ids::MemberId;

/// Tables whose rows block deleting a member.
const REFERENCES: &[(&str, &str)] = &[
    ("entries", "member_id"),
    ("jobs", "member_id"),
    ("agreement_shares", "member_id"),
    ("entry_share_overrides", "member_id"),
    ("account_members", "member_id"),
    ("recurring_templates", "member_id"),
    ("offset_rules", "credit_member_id"),
    ("offset_rules", "debit_member_id"),
];

/// Household members.
pub struct Members;

impl Members {
    /// Every member, ordered by id.
    pub fn list(conn: &Connection) -> Result<Vec<Member>, StoreError> {
        query(conn, &format!("SELECT {MEMBER_COLS} FROM members ORDER BY id"), [], rows::member)
    }

    /// Inserts a member.
    pub fn create(conn: &Connection, input: &MemberInput) -> Result<MemberId, StoreError> {
        let sql = "INSERT INTO members (name_ct, color, is_active) VALUES (?, ?, ?) RETURNING id";
        insert_id(conn, sql, params![input.name.as_str(), input.color, input.is_active]).map(MemberId)
    }

    /// Replaces a member's attributes.
    pub fn update(conn: &Connection, id: MemberId, input: &MemberInput) -> Result<(), StoreError> {
        let sql = "UPDATE members SET name_ct = ?, color = ?, is_active = ? WHERE id = ?";
        touch_one(conn, sql, params![input.name.as_str(), input.color, input.is_active, id.0], "member")
    }

    /// Deletes an unreferenced member; deactivate instead to keep history.
    pub fn delete(conn: &Connection, id: MemberId) -> Result<(), StoreError> {
        unreferenced(conn, REFERENCES, id.0, "member")?;
        conn.execute("DELETE FROM project_members WHERE member_id = ?", [id.0])?;
        conn.execute("DELETE FROM income_overrides WHERE member_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM members WHERE id = ?", [id.0], "member")
    }
}
