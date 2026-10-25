use super::Seals;
use crate::inputs::RecurringInput;
use crate::rows::{self, RECURRING_COLS};
use crate::sql::{insert_id, query, require, touch_one};
use crate::StoreError;
use chrono::{Duration, NaiveDate};
use duckdb::{Connection, ToSql, params, params_from_iter};
use jizifin_domain::entities::RecurringTemplate;
use jizifin_domain::ids::{EntryId, RecurringId};
use jizifin_domain::Month;

/// Recurring templates and their idempotent materialization into entries.
pub struct Recurring;

impl Recurring {
    /// Every template.
    pub fn list(conn: &Connection) -> Result<Vec<RecurringTemplate>, StoreError> {
        query(conn, &format!("SELECT {RECURRING_COLS} FROM recurring_templates ORDER BY id"), [], rows::recurring)
    }

    /// Inserts a template.
    pub fn create(conn: &Connection, input: &RecurringInput) -> Result<RecurringId, StoreError> {
        Self::check(conn, input)?;
        let sql = "INSERT INTO recurring_templates (kind, amount_cents, member_id, category_id, account_id, day_of_month, \
                   frequency, valid_from, valid_to, name_ct, is_active) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id";
        insert_id(conn, sql, params_from_iter(Self::params(input))).map(RecurringId)
    }

    /// Replaces a template. Already generated entries are kept.
    pub fn update(conn: &Connection, id: RecurringId, input: &RecurringInput) -> Result<(), StoreError> {
        Self::check(conn, input)?;
        let sql = "UPDATE recurring_templates SET kind = ?, amount_cents = ?, member_id = ?, category_id = ?, account_id = ?, \
                   day_of_month = ?, frequency = ?, valid_from = ?, valid_to = ?, name_ct = ?, is_active = ? WHERE id = ?";
        let mut p = Self::params(input);
        p.push(Box::new(id.0));
        touch_one(conn, sql, params_from_iter(p), "recurring template")
    }

    /// Deletes a template; generated entries stay and lose the link.
    pub fn delete(conn: &Connection, id: RecurringId) -> Result<(), StoreError> {
        conn.execute("UPDATE entries SET recurring_id = NULL WHERE recurring_id = ?", [id.0])?;
        touch_one(conn, "DELETE FROM recurring_templates WHERE id = ?", [id.0], "recurring template")
    }

    /// Inserts every missing occurrence in `[min(first of month, today − grace), today]`,
    /// skipping sealed months. Safe to run repeatedly.
    pub fn materialize(conn: &Connection, today: NaiveDate, grace_days: i64) -> Result<Vec<EntryId>, StoreError> {
        let from = Month::of(today).first_day().min(today - Duration::days(grace_days));
        let templates: Vec<RecurringTemplate> = Self::list(conn)?.into_iter().filter(|t| t.is_active).collect();
        let due = templates.iter().flat_map(|t| Self::due_dates(t, from, today).into_iter().map(move |d| (t, d)));
        let open = due.filter(|(_, d)| Seals::guard(conn, &[*d]).is_ok()).collect::<Vec<_>>();
        open.into_iter().filter_map(|(t, d)| Self::insert_occurrence(conn, t, d).transpose()).collect()
    }

    fn due_dates(template: &RecurringTemplate, from: NaiveDate, to: NaiveDate) -> Vec<NaiveDate> {
        let months = std::iter::successors(Some(Month::of(from)), |m| Some(m.next())).take_while(|m| m.first_day() <= to);
        months.flat_map(|m| template.occurrences(m)).filter(|d| from <= *d && *d <= to).collect()
    }

    fn insert_occurrence(conn: &Connection, t: &RecurringTemplate, date: NaiveDate) -> Result<Option<EntryId>, StoreError> {
        let sql = "INSERT INTO entries (kind, entry_date, amount_cents, member_id, category_id, account_id, recurring_id, name_ct) \
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING RETURNING id";
        let p = params![t.kind.label(), date, t.amount.value(), t.member.0, t.category.0, t.account.map(|a| a.0), t.id.0, t.name.as_str()];
        let ids = query(conn, sql, p, |r| r.get::<_, i64>(0))?;
        Ok(ids.first().copied().map(EntryId))
    }

    fn check(conn: &Connection, input: &RecurringInput) -> Result<(), StoreError> {
        input.validate()?;
        require(conn, "members", input.member.0)?;
        require(conn, "categories", input.category.0)?;
        input.account.map_or(Ok(()), |a| require(conn, "accounts", a.0))
    }

    fn params(input: &RecurringInput) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(input.kind.label()),
            Box::new(input.amount.value()),
            Box::new(input.member.0),
            Box::new(input.category.0),
            Box::new(input.account.map(|a| a.0)),
            Box::new(input.day_of_month),
            Box::new(input.recurrence.label()),
            Box::new(input.valid_from),
            Box::new(input.valid_to),
            Box::new(input.name.as_str().to_owned()),
            Box::new(input.is_active),
        ]
    }
}
