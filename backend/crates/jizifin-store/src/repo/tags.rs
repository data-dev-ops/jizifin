use crate::inputs::TagInput;
use crate::rows::{self, TAG_COLS};
use crate::sql::{exists, insert_id, opt_ct, query, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::Tag;
use jizifin_domain::ids::TagId;

/// Event tags with optional active windows.
pub struct Tags;

impl Tags {
    /// Every tag.
    pub fn list(conn: &Connection) -> Result<Vec<Tag>, StoreError> {
        query(conn, &format!("SELECT {TAG_COLS} FROM tags ORDER BY id"), [], rows::tag)
    }

    /// One tag, or `Invalid` when it does not exist.
    pub fn get(conn: &Connection, id: TagId) -> Result<Tag, StoreError> {
        let found = query(conn, &format!("SELECT {TAG_COLS} FROM tags WHERE id = ?"), [id.0], rows::tag)?;
        found.into_iter().next().ok_or_else(|| StoreError::Invalid(format!("tag {id} does not exist")))
    }

    /// Inserts a tag.
    pub fn create(conn: &Connection, input: &TagInput) -> Result<TagId, StoreError> {
        input.validate()?;
        let sql = "INSERT INTO tags (name_ct, description_ct, color, valid_from, valid_to) VALUES (?, ?, ?, ?, ?) RETURNING id";
        let p = params![input.name.as_str(), opt_ct(&input.description), input.color, input.valid_from, input.valid_to];
        insert_id(conn, sql, p).map(TagId)
    }

    /// Replaces a tag; rejects a window that would orphan tagged entries.
    pub fn update(conn: &Connection, id: TagId, input: &TagInput) -> Result<(), StoreError> {
        input.validate()?;
        Self::reject_orphans(conn, id, input)?;
        let sql = "UPDATE tags SET name_ct = ?, description_ct = ?, color = ?, valid_from = ?, valid_to = ? WHERE id = ?";
        let p = params![input.name.as_str(), opt_ct(&input.description), input.color, input.valid_from, input.valid_to, id.0];
        touch_one(conn, sql, p, "tag")
    }

    /// Deletes a tag; its entries stay and lose the link.
    pub fn delete(conn: &Connection, id: TagId) -> Result<(), StoreError> {
        conn.execute("UPDATE entries SET tag_id = NULL WHERE tag_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM tags WHERE id = ?", [id.0], "tag")
    }

    fn reject_orphans(conn: &Connection, id: TagId, input: &TagInput) -> Result<(), StoreError> {
        let sql = "SELECT 1 FROM entries WHERE tag_id = ? AND (entry_date < COALESCE(?, entry_date) OR entry_date > COALESCE(?, entry_date)) LIMIT 1";
        match exists(conn, sql, params![id.0, input.valid_from, input.valid_to])? {
            true => Err(StoreError::Invalid("tagged entries fall outside the new window".into())),
            false => Ok(()),
        }
    }
}
