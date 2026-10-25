//! Effective monthly income per member: overrides beat contracts; one-off income adds on top.

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
        self.active_jobs(member).map(|j| self.normalizer.monthly(j.amount, j.frequency)).sum()
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
