use crate::inputs::JobInput;
use crate::rows::{self, INCOME_OVERRIDE_COLS, JOB_COLS};
use crate::sql::{insert_id, opt_ct, query, require, touch_one};
use crate::StoreError;
use duckdb::{Connection, params};
use jizifin_domain::entities::{IncomeOverride, Job};
use jizifin_domain::ids::{JobId, MemberId};
use jizifin_domain::{Month, Period};

/// Employment contracts.
pub struct Jobs;

impl Jobs {
    /// Every job.
    pub fn list(conn: &Connection) -> Result<Vec<Job>, StoreError> {
        query(conn, &format!("SELECT {JOB_COLS} FROM jobs ORDER BY member_id, valid_from, id"), [], rows::job)
    }

    /// Inserts a job.
    pub fn create(conn: &Connection, input: &JobInput) -> Result<JobId, StoreError> {
        Self::check(conn, input)?;
        let sql = "INSERT INTO jobs (member_id, name_ct, amount_cents, frequency, valid_from, valid_to, notes_ct) \
                   VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id";
        insert_id(conn, sql, duckdb::params_from_iter(Self::params(input))).map(JobId)
    }

    /// Replaces a job.
    pub fn update(conn: &Connection, id: JobId, input: &JobInput) -> Result<(), StoreError> {
        Self::check(conn, input)?;
        let sql = "UPDATE jobs SET member_id = ?, name_ct = ?, amount_cents = ?, frequency = ?, valid_from = ?, \
                   valid_to = ?, notes_ct = ? WHERE id = ?";
        let mut p = Self::params(input);
        p.push(Box::new(id.0));
        touch_one(conn, sql, duckdb::params_from_iter(p), "job")
    }

    /// Deletes a job.
    pub fn delete(conn: &Connection, id: JobId) -> Result<(), StoreError> {
        touch_one(conn, "DELETE FROM jobs WHERE id = ?", [id.0], "job")
    }

    fn check(conn: &Connection, input: &JobInput) -> Result<(), StoreError> {
        input.validate()?;
        require(conn, "members", input.member.0)
    }

    fn params(input: &JobInput) -> Vec<Box<dyn duckdb::ToSql>> {
        vec![
            Box::new(input.member.0),
            Box::new(input.name.as_str().to_owned()),
            Box::new(input.amount.value()),
            Box::new(input.frequency.label()),
            Box::new(input.valid_from),
            Box::new(input.valid_to),
            Box::new(opt_ct(&input.notes).map(str::to_owned)),
        ]
    }
}

/// Month-specific salary replacements.
pub struct IncomeOverrides;

impl IncomeOverrides {
    /// Overrides inside `period`.
    pub fn list(conn: &Connection, period: Period) -> Result<Vec<IncomeOverride>, StoreError> {
        let sql = format!("SELECT {INCOME_OVERRIDE_COLS} FROM income_overrides WHERE month >= date_trunc('month', ?::DATE) AND month <= ? ORDER BY month, member_id");
        query(conn, &sql, params![period.from, period.to], rows::income_override)
    }

    /// Inserts or replaces the override of `(member, month)`.
    pub fn upsert(conn: &Connection, input: &IncomeOverride) -> Result<(), StoreError> {
        require(conn, "members", input.member.0)?;
        let sql = "INSERT OR REPLACE INTO income_overrides (member_id, month, amount_cents, note_ct) VALUES (?, ?, ?, ?)";
        conn.execute(sql, params![input.member.0, input.month.first_day(), input.amount.value(), opt_ct(&input.note)])?;
        Ok(())
    }

    /// Deletes the override of `(member, month)`.
    pub fn delete(conn: &Connection, member: MemberId, month: Month) -> Result<(), StoreError> {
        let sql = "DELETE FROM income_overrides WHERE member_id = ? AND month = ?";
        touch_one(conn, sql, params![member.0, month.first_day()], "income override")
    }
}
