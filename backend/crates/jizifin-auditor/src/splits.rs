//! Effective split per expense category for a month, and whole-percent split proposals.

use crate::allocation::{LargestRemainderAllocator, TieSeed, Weight};
use crate::income::IncomeResolver;
use crate::invariant::{Invariant, InvariantSet};
use crate::ratios::SalaryRatioCalculator;
use crate::shares::ShareSource;
use crate::{AuditScope, Snapshot};
use jizifin_domain::entities::{Agreement, Category, CategoryKind, PayMode};
use jizifin_domain::ids::{AgreementId, CategoryId, MemberId};
use chrono::NaiveDate;
use jizifin_domain::Month;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tolerance on percentage totals.
const PCT_EPSILON: f64 = 1e-4;

/// How one category splits in a month, as the settlement applies it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CategorySplit {
    /// Category.
    pub category: CategoryId,
    /// Rule that applies (personal, income ratio, agreement or equal).
    pub source: ShareSource,
    /// Agreement in force, when the source is an agreement.
    pub agreement: Option<AgreementId>,
    /// True when that agreement is a date-bounded override.
    pub is_override: bool,
    /// Days of the month the agreement covers, when that is only part of it; entries on the other days use the baseline split.
    pub partial: Option<Coverage>,
    /// Percent per member; empty for personal pay (the payer bears everything).
    pub pct: BTreeMap<MemberId, f64>,
}

/// First and last day of the month an agreement is effective.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Coverage {
    /// First effective day inside the month.
    pub from: NaiveDate,
    /// Last effective day inside the month.
    pub to: NaiveDate,
}

/// Every expense category's split for a month.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SplitOverview {
    /// Month.
    pub month: Month,
    /// One row per expense category.
    pub categories: Vec<CategorySplit>,
}

/// Resolves [`SplitOverview`].
pub struct SplitOverviewCalculator<'a> {
    snap: &'a Snapshot,
    month: Month,
}

impl<'a> SplitOverviewCalculator<'a> {
    /// Calculator for `scope.month`.
    pub fn new(snap: &'a Snapshot, scope: &AuditScope) -> Self {
        Self { snap, month: scope.month }
    }

    /// Splits of all expense categories.
    pub fn calculate(&self) -> SplitOverview {
        let expense = self.snap.categories.values().filter(|c| c.kind == CategoryKind::Expense);
        SplitOverview { month: self.month, categories: expense.map(|c| self.split(c)).collect() }
    }

    fn split(&self, category: &Category) -> CategorySplit {
        let row = |source, agreement: Option<&Agreement>, pct| CategorySplit {
            category: category.id,
            source,
            agreement: agreement.map(|a| a.id),
            is_override: agreement.is_some_and(Agreement::is_bounded),
            partial: agreement.and_then(|a| self.partial_coverage(a)),
            pct,
        };
        match (category.pay_mode, self.agreement(category.id)) {
            (PayMode::Personal, _) => row(ShareSource::Personal, None, BTreeMap::new()),
            (PayMode::IncomeRatio, _) if self.has_income() => row(ShareSource::IncomeRatio, None, self.income_pct()),
            (_, Some(a)) => row(ShareSource::Agreement, Some(a), a.shares.iter().map(|s| (s.member, s.pct)).collect()),
            (_, None) => row(ShareSource::Equal, None, self.equal_pct()),
        }
    }

    /// The agreement's window inside the month, unless it covers every day.
    fn partial_coverage(&self, agreement: &Agreement) -> Option<Coverage> {
        let from = agreement.valid_from.max(self.month.first_day());
        let to = agreement.valid_to.map_or(self.month.last_day(), |end| end.min(self.month.last_day()));
        (from > self.month.first_day() || to < self.month.last_day()).then_some(Coverage { from, to })
    }

    /// Highest-precedence agreement overlapping the month.
    fn agreement(&self, category: CategoryId) -> Option<&'a Agreement> {
        let month = self.month.period();
        let in_month = |a: &&'a Agreement| a.category == category && month.overlaps(a.valid_from, a.valid_to);
        self.snap.agreements.iter().filter(in_month).max_by_key(|a| a.precedence())
    }

    fn has_income(&self) -> bool {
        IncomeResolver::new(self.snap, self.month).weights().iter().any(|w| w.weight > 0.0)
    }

    fn income_pct(&self) -> BTreeMap<MemberId, f64> {
        normalized(&IncomeResolver::new(self.snap, self.month).weights())
    }

    fn equal_pct(&self) -> BTreeMap<MemberId, f64> {
        let weights: Vec<Weight> = self.snap.active_members().into_iter().map(|member| Weight { member, weight: 1.0 }).collect();
        normalized(&weights)
    }
}

/// Weights as percentages of their sum.
fn normalized(weights: &[Weight]) -> BTreeMap<MemberId, f64> {
    let total: f64 = weights.iter().map(|w| w.weight).sum();
    weights.iter().map(|w| (w.member, if total > 0.0 { w.weight / total * 100.0 } else { 0.0 })).collect()
}

/// Strategy for a proposed split.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitStrategy {
    /// Equal whole percentages.
    Even,
    /// Proportional to effective income.
    Salary,
}

/// Whole-percent proposal over the scoped members.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SplitProposal {
    /// Strategy used.
    pub strategy: SplitStrategy,
    /// Percent per member, summing to 100.
    pub pct: BTreeMap<MemberId, i64>,
}

/// Builds [`SplitProposal`]s.
pub struct SplitProposer<'a> {
    snap: &'a Snapshot,
    scope: &'a AuditScope,
}

impl<'a> SplitProposer<'a> {
    /// Proposer for `scope` (members filter = participants).
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope }
    }

    /// Proposal for `strategy`.
    pub fn propose(&self, strategy: SplitStrategy) -> SplitProposal {
        let pct = match strategy {
            SplitStrategy::Salary => SalaryRatioCalculator::new(self.snap, self.scope).calculate().pct,
            SplitStrategy::Even => self.even(),
        };
        SplitProposal { strategy, pct }
    }

    fn even(&self) -> BTreeMap<MemberId, i64> {
        let members = match &self.scope.members {
            Some(set) => set.iter().copied().collect(),
            None => self.snap.active_members(),
        };
        let weights: Vec<Weight> = members.into_iter().map(|member| Weight { member, weight: 1.0 }).collect();
        let seed = TieSeed::named(format!("even:{}", self.scope.month));
        LargestRemainderAllocator.allocate(100, &weights, &seed).iter().collect()
    }
}

/// Invariant set applied to [`SplitOverview`].
pub fn overview_invariants() -> InvariantSet<SplitOverview> {
    InvariantSet::new(vec![Box::new(SharedSplitsTotal100)])
}

/// Every non-personal split with members totals 100 %.
struct SharedSplitsTotal100;

impl Invariant<SplitOverview> for SharedSplitsTotal100 {
    fn id(&self) -> &'static str {
        "shared_splits_total_100"
    }
    fn holds(&self, o: &SplitOverview, _: &Snapshot) -> bool {
        o.categories.iter().filter(|c| !c.pct.is_empty()).all(|c| (c.pct.values().sum::<f64>() - 100.0).abs() <= PCT_EPSILON)
    }
}

/// Invariant set applied to [`SplitProposal`].
pub fn proposal_invariants() -> InvariantSet<SplitProposal> {
    InvariantSet::new(vec![Box::new(ProposalTotals100)])
}

/// Proposals total exactly 100 whenever any member is present.
struct ProposalTotals100;

impl Invariant<SplitProposal> for ProposalTotals100 {
    fn id(&self) -> &'static str {
        "proposal_totals_100"
    }
    fn holds(&self, p: &SplitProposal, _: &Snapshot) -> bool {
        p.pct.is_empty() || p.pct.values().sum::<i64>() == 100
    }
}
