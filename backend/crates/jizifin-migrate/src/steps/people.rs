//! Members, jobs (including legacy `SALARY` income conversion) and salary overrides.

use super::{Steps, ct, date, month, opt_ct, opt_date};
use crate::error::MigrateError;
use crate::legacy::LegacyIncome;
use chrono::Duration;
use jizifin_domain::entities::{Frequency, IncomeOverride};
use jizifin_domain::ids::MemberId;
use jizifin_domain::{Cents, Month};
use jizifin_store::inputs::{JobInput, MemberInput};
use jizifin_store::repo::{IncomeOverrides, Jobs, Members};
use std::collections::BTreeSet;

impl Steps<'_> {
    pub(super) fn members(&mut self) -> Result<(), MigrateError> {
        let users = self.legacy.users()?;
        for user in &users {
            let input = MemberInput { name: ct(&user.name)?, color: user.color.clone(), is_active: user.is_active };
            self.ids.members.insert(user.name.clone(), Members::create(self.conn, &input)?);
        }
        self.report.count("members", users.len());
        Ok(())
    }

    /// Jobs, salary rows converted to jobs for members without any, and month overrides.
    pub(super) fn income_streams(&mut self) -> Result<(), MigrateError> {
        let with_jobs = self.jobs()?;
        self.salary_jobs(&with_jobs)?;
        self.salary_overrides()
    }

    fn jobs(&mut self) -> Result<BTreeSet<MemberId>, MigrateError> {
        let jobs = self.legacy.jobs()?;
        let mut with_jobs = BTreeSet::new();
        for job in &jobs {
            let member = self.ids.member(&job.who)?;
            with_jobs.insert(member);
            if job.is_active {
                let input = JobInput {
                    member, name: ct(&job.name)?, amount: Cents::new(job.amount), frequency: Frequency::parse(&job.frequency)?,
                    valid_from: date(&job.start)?, valid_to: opt_date(job.end.as_deref())?, notes: opt_ct(job.notes.as_deref())?,
                };
                Jobs::create(self.conn, &input)?;
            }
        }
        self.report.count("jobs", jobs.iter().filter(|j| j.is_active).count());
        self.note_inactive_jobs(jobs.iter().filter(|j| !j.is_active).count());
        Ok(with_jobs)
    }

    fn note_inactive_jobs(&mut self, skipped: usize) {
        if skipped > 0 {
            self.report.note(format!("{skipped} inactive jobs dropped (they never counted towards salary)"));
        }
    }

    /// Legacy carry-forward salary rows become monthly jobs; one job per amount change.
    fn salary_jobs(&mut self, with_jobs: &BTreeSet<MemberId>) -> Result<(), MigrateError> {
        let salary: Vec<LegacyIncome> = self.legacy.income()?.into_iter().filter(|i| i.category == self.markers.salary).collect();
        let mut created = 0;
        for member in self.ids.members.values().copied().filter(|m| !with_jobs.contains(m)).collect::<Vec<_>>() {
            created += self.member_salary_jobs(member, &salary)?;
        }
        self.report.count("jobs from legacy SALARY income", created);
        Ok(())
    }

    fn member_salary_jobs(&mut self, member: MemberId, salary: &[LegacyIncome]) -> Result<usize, MigrateError> {
        let rows: Vec<&LegacyIncome> = salary.iter().filter(|i| self.ids.member(&i.who).is_ok_and(|m| m == member)).collect();
        let changes = Self::amount_changes(&rows);
        for (index, row) in changes.iter().enumerate() {
            let next_start = changes.get(index + 1).map(|n| date(&n.date).map(|d| Month::of(d).first_day())).transpose()?;
            let input = JobInput {
                member, name: ct(&row.name)?, amount: Cents::new(row.amount), frequency: Frequency::Monthly,
                valid_from: Month::of(date(&row.date)?).first_day(), valid_to: next_start.map(|d| d - Duration::days(1)), notes: None,
            };
            Jobs::create(self.conn, &input)?;
        }
        Ok(changes.len())
    }

    fn amount_changes<'r>(rows: &[&'r LegacyIncome]) -> Vec<&'r LegacyIncome> {
        let mut changes: Vec<&LegacyIncome> = Vec::new();
        for row in rows {
            if changes.last().is_none_or(|last| last.amount != row.amount) {
                changes.push(row);
            }
        }
        changes
    }

    fn salary_overrides(&mut self) -> Result<(), MigrateError> {
        let overrides = self.legacy.salary_overrides()?;
        for o in &overrides {
            let input = IncomeOverride { member: self.ids.member(&o.user)?, month: month(&o.month)?, amount: Cents::new(o.amount), note: opt_ct(o.note.as_deref())? };
            IncomeOverrides::upsert(self.conn, &input)?;
        }
        self.report.count("income overrides", overrides.len());
        Ok(())
    }
}
