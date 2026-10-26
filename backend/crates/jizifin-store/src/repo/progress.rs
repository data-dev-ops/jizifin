//! Project funding progress and completion forecast from the average monthly payment.

use crate::sql::query;
use crate::StoreError;
use chrono::{Datelike, Months, NaiveDate};
use duckdb::{Connection, Row};
use jizifin_domain::ids::ProjectId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

/// When a project is expected to reach its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Completion {
    /// Spent at least the target.
    Completed,
    /// Nothing paid yet, so no pace to project.
    Indefinite,
    /// Target reached around `date`, `months` from now, at the current pace.
    Projected {
        /// Forecast date.
        date: NaiveDate,
        /// Whole months until then.
        months: i64,
    },
}

/// Funding state of one project.
#[derive(Debug, Serialize)]
pub struct ProjectProgress {
    /// Project.
    pub project: ProjectId,
    /// Target.
    pub target: Cents,
    /// Σ expenses booked on the project.
    pub spent: Cents,
    /// Entry count.
    pub count: i64,
    /// `spent / target × 100`, capped at 100; absent without a target.
    pub progress_pct: Option<f64>,
    /// Mean of the months that had payments.
    pub monthly_avg: Cents,
    /// Most recent payment date.
    pub last_payment: Option<NaiveDate>,
    /// Forecast.
    pub completion: Completion,
}

/// One project-month aggregate row.
struct MonthRow {
    project: i64,
    target: i64,
    total: i64,
    count: i64,
    last: Option<NaiveDate>,
}

/// Progress of every project as of `today`.
pub fn project_progress(conn: &Connection, today: NaiveDate) -> Result<Vec<ProjectProgress>, StoreError> {
    let sql = "SELECT p.id, p.target_cents, CAST(COALESCE(SUM(e.amount_cents), 0) AS BIGINT), COUNT(e.id), MAX(e.entry_date) \
        FROM projects p LEFT JOIN entries e ON e.project_id = p.id AND e.kind = 'expense' \
        GROUP BY p.id, p.target_cents, date_trunc('month', e.entry_date) ORDER BY p.id";
    let rows = query(conn, sql, [], month_row)?;
    let mut by_project: BTreeMap<i64, Vec<MonthRow>> = BTreeMap::new();
    rows.into_iter().for_each(|r| by_project.entry(r.project).or_default().push(r));
    Ok(by_project.into_values().map(|months| Forecast { months, today }.progress()).collect())
}

fn month_row(r: &Row<'_>) -> duckdb::Result<MonthRow> {
    Ok(MonthRow { project: r.get(0)?, target: r.get(1)?, total: r.get(2)?, count: r.get(3)?, last: r.get(4)? })
}

/// Monthly rows of one project (a project without entries has one empty row).
struct Forecast {
    months: Vec<MonthRow>,
    today: NaiveDate,
}

impl Forecast {
    fn progress(&self) -> ProjectProgress {
        let (target, spent) = (Cents::new(self.months[0].target), self.spent());
        let monthly_avg = self.monthly_avg();
        ProjectProgress {
            project: ProjectId(self.months[0].project),
            target,
            spent,
            count: self.months.iter().map(|m| m.count).sum(),
            progress_pct: target.is_positive().then(|| (spent.value() as f64 / target.value() as f64 * 100.0).min(100.0)),
            monthly_avg,
            last_payment: self.months.iter().filter_map(|m| m.last).max(),
            completion: self.completion(target - spent, monthly_avg),
        }
    }

    fn spent(&self) -> Cents {
        Cents::new(self.months.iter().map(|m| m.total).sum())
    }

    fn paid_months(&self) -> impl Iterator<Item = &MonthRow> {
        self.months.iter().filter(|m| m.count > 0)
    }

    fn monthly_avg(&self) -> Cents {
        let paid = self.paid_months().count();
        match paid {
            0 => Cents::ZERO,
            n => Cents::new((self.paid_months().map(|m| m.total).sum::<i64>() as f64 / n as f64).round_ties_even() as i64),
        }
    }

    fn completion(&self, remaining: Cents, avg: Cents) -> Completion {
        match (remaining.is_positive(), avg.is_positive()) {
            (false, _) => Completion::Completed,
            (true, false) => Completion::Indefinite,
            (true, true) => {
                let months = (remaining.value() + avg.value() - 1) / avg.value();
                Completion::Projected { date: self.months_ahead(months), months }
            }
        }
    }

    fn months_ahead(&self, months: i64) -> NaiveDate {
        let first = Month::of(self.today).first_day() + Months::new(u32::try_from(months).unwrap_or(u32::MAX));
        let month = Month::of(first);
        month.first_day().with_day(month.clamp_day(self.today.day())).unwrap_or(first)
    }
}
