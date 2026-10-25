use crate::inputs::ProjectInput;
use crate::rows::{self, PROJECT_COLS};
use crate::sql::{insert_id, query, require, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::Project;
use jizifin_domain::ids::{MemberId, ProjectId};

/// Projects with members.
pub struct Projects;

impl Projects {
    /// Every project, fully populated.
    pub fn all(conn: &Connection) -> Result<Vec<Project>, StoreError> {
        let mut projects = query(conn, &format!("SELECT {PROJECT_COLS} FROM projects ORDER BY id"), [], rows::project)?;
        let members: Vec<(i64, i64)> =
            query(conn, "SELECT project_id, member_id FROM project_members ORDER BY 1, 2", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        for project in &mut projects {
            project.members = members.iter().filter(|(p, _)| *p == project.id.0).map(|(_, m)| MemberId(*m)).collect();
        }
        Ok(projects)
    }

    /// Inserts a project.
    pub fn create(conn: &Connection, input: &ProjectInput) -> Result<ProjectId, StoreError> {
        let sql = "INSERT INTO projects (name_ct, color, target_cents, target_date) VALUES (?, ?, ?, ?) RETURNING id";
        let p = params![input.name.as_str(), input.color, input.target.value(), input.target_date];
        let id = ProjectId(insert_id(conn, sql, p)?);
        Self::write_members(conn, id, &input.members)?;
        Ok(id)
    }

    /// Replaces a project and its members.
    pub fn update(conn: &Connection, id: ProjectId, input: &ProjectInput) -> Result<(), StoreError> {
        let sql = "UPDATE projects SET name_ct = ?, color = ?, target_cents = ?, target_date = ? WHERE id = ?";
        let p = params![input.name.as_str(), input.color, input.target.value(), input.target_date, id.0];
        touch_one(conn, sql, p, "project")?;
        Self::write_members(conn, id, &input.members)
    }

    /// Deletes a project; its entries stay and lose the link.
    pub fn delete(conn: &Connection, id: ProjectId) -> Result<(), StoreError> {
        conn.execute("UPDATE entries SET project_id = NULL WHERE project_id = ?", [id.0])?;
        conn.execute("DELETE FROM project_members WHERE project_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM projects WHERE id = ?", [id.0], "project")
    }

    fn write_members(conn: &Connection, id: ProjectId, members: &[MemberId]) -> Result<(), StoreError> {
        members.iter().try_for_each(|m| require(conn, "members", m.0))?;
        conn.execute("DELETE FROM project_members WHERE project_id = ?", [id.0])?;
        let mut stmt = conn.prepare_cached("INSERT INTO project_members (project_id, member_id) VALUES (?, ?)")?;
        members.iter().try_for_each(|m| stmt.execute(params![id.0, m.0]).map(|_| ()))?;
        Ok(())
    }
}
