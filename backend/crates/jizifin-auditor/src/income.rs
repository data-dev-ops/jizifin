//! Effective monthly income per member: overrides beat contracts; one-off income adds on top.

use chrono::Datelike;
use jizifin_domain::entities::{EntryKind, Frequency, Job};
use jizifin_domain::ids::MemberId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::Snapshot;
use crate::allocation::Weight;

/// Converts a contract amount to its monthly equivalent.
#[derive(Clone, Copy, Debug, Default)]
pub struct SalaryNormalizer;

impl SalaryNormalizer {
    /// Monthly equivalent, rounding ties to even.
    pub fn monthly(&self, amount: Cents, frequency: Frequency) -> Cents {
        let yearly = amount.value() * frequency.periods_per_year();
        Cents::new((yearly as f64 / 12.0).round_ties_even() as i64)
    }
}

/// Income breakdown for one member in one month.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemberIncome {
    /// Member.
    pub member: MemberId,
    /// Σ normalized active contracts.
    pub contract: Cents,
    /// Contract or override, whichever applies.
    pub base: Cents,
    /// Σ income entries booked this month.
    pub one_off: Cents,
    /// `base + one_off`.
    pub total: Cents,
    /// True when an override replaced the contract salary.
    pub has_override: bool,
}

/// Resolves [`MemberIncome`] for every member of a snapshot.
pub struct IncomeResolver<'a> {
    snap: &'a Snapshot,
    month: Month,
    normalizer: SalaryNormalizer,
}

impl<'a> IncomeResolver<'a> {
    /// Resolver for `month`.
    pub fn new(snap: &'a Snapshot, month: Month) -> Self {
        Self { snap, month, normalizer: SalaryNormalizer }
    }

    /// Income per active member.
    pub fn resolve(&self) -> BTreeMap<MemberId, MemberIncome> {
        self.snap.active_members().into_iter().map(|m| (m, self.member_income(m))).collect()
    }

    /// Income totals as allocation weights (negative totals count as zero).
    pub fn weights(&self) -> Vec<Weight> {
        self.resolve()
            .values()
            .map(|i| Weight { member: i.member, weight: i.total.value().max(0) as f64 })
            .collect()
    }

    fn member_income(&self, member: MemberId) -> MemberIncome {
        let contract = self.contract(member);
        let override_amount = self.override_amount(member);
        let base = override_amount.unwrap_or(contract);
        let one_off = self.one_off(member);
        MemberIncome { member, contract, base, one_off, total: base + one_off, has_override: override_amount.is_some() }
    }

    fn contract(&self, member: MemberId) -> Cents {
        self.active_jobs(member).map(|j| self.payout(j)).sum()
    }

    /// Cash paid by `job` this month: days worked prorated, plus the first-salary catch-up after a switch.
    fn payout(&self, job: &Job) -> Cents {
        let monthly = self.normalizer.monthly(job.amount, job.frequency);
        let switch = self.switch_month(job);
        if switch == Some(self.month) {
            return Cents::ZERO;
        }
        let catch_up = switch.map_or(Cents::ZERO, |m| Self::prorate(monthly, m, Self::days_worked(job, m)));
        Self::prorate(monthly, self.month, Self::days_worked(job, self.month)) + catch_up
    }

    /// Days of `month` the contract covers, bounded by both `valid_from` and `valid_to`.
    fn days_worked(job: &Job, month: Month) -> i64 {
        let first = job.valid_from.max(month.first_day());
        let last = job.valid_to.map_or(month.last_day(), |end| end.min(month.last_day()));
        ((last - first).num_days() + 1).max(0)
    }

    /// Month in which `job` replaced another of the member's jobs mid-month; its first salary lands the month after.
    fn switch_month(&self, job: &Job) -> Option<Month> {
        let start = Month::of(job.valid_from);
        let mid_month = job.valid_from.day() > 1;
        let replaced = self.snap.jobs.iter().any(|p| p.member == job.member && Self::ended_in_before(p, start, job));
        (mid_month && replaced).then_some(start)
    }

    fn ended_in_before(previous: &Job, month: Month, job: &Job) -> bool {
        previous.valid_to.is_some_and(|end| month.period().contains(end) && end < job.valid_from)
    }

    /// `days` of `month` out of its length, rounded half up; a full month is exactly `monthly`.
    fn prorate(monthly: Cents, month: Month, days: i64) -> Cents {
        let length = i64::from(month.last_day().day());
        Cents::new((monthly.value() * days * 2 + length) / (length * 2))
    }

    fn active_jobs(&self, member: MemberId) -> impl Iterator<Item = &Job> {
        self.snap.jobs.iter().filter(move |j| j.member == member && j.active_in(self.month))
    }

    fn override_amount(&self, member: MemberId) -> Option<Cents> {
        let found = self.snap.income_overrides.iter().find(|o| o.member == member && o.month == self.month);
        found.map(|o| o.amount)
    }

    fn one_off(&self, member: MemberId) -> Cents {
        let entries = self.snap.entries_in(EntryKind::Income, self.month);
        entries.filter(|e| e.member == Some(member)).map(|e| e.amount).sum()
    }
}
