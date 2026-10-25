//! Budget usage and proposals.

use crate::invariant::{Invariant, InvariantSet};
use crate::{AuditScope, Snapshot};
use jizifin_domain::entities::{Budget, EntryKind};
use jizifin_domain::ids::CategoryId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

/// Months of history a proposal averages over.
pub const PROPOSAL_LOOKBACK_MONTHS: u32 = 3;

/// Proposals are rounded up to whole currency units.
const WHOLE_UNIT: i64 = 100;

/// Spending vs limit for one budgeted category.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BudgetStatus {
    /// Category.
    pub category: CategoryId,
    /// Limit.
    pub limit: Cents,
    /// Spent this month.
    pub spent: Cents,
    /// `limit − spent`.
    pub remaining: Cents,
    /// `spent / limit` in percent, one decimal.
    pub pct_used: f64,
}

/// Proposed limit for one category.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BudgetProposal {
    /// Category.
    pub category: CategoryId,
    /// Spend in each lookback month, oldest first.
    pub history: Vec<Cents>,
    /// Mean of `history`, rounded up to a whole unit.
    pub proposed: Cents,
}

/// Spend per category and month for expense entries in the snapshot.
struct SpendIndex(BTreeMap<(CategoryId, Month), Cents>);

impl SpendIndex {
    fn build(snap: &Snapshot) -> Self {
        let mut index = BTreeMap::new();
        let expenses = snap.entries.iter().filter(|e| e.kind == EntryKind::Expense);
        for (entry, category) in expenses.filter_map(|e| e.category.map(|c| (e, c))) {
            *index.entry((category, Month::of(entry.date))).or_default() += entry.amount;
        }
        Self(index)
    }

    fn spent(&self, category: CategoryId, month: Month) -> Cents {
        self.0.get(&(category, month)).copied().unwrap_or_default()
    }

    fn categories(&self) -> impl Iterator<Item = CategoryId> + '_ {
        let mut seen: Vec<CategoryId> = self.0.keys().map(|(c, _)| *c).collect();
        seen.dedup();
        seen.into_iter()
    }
}

/// Evaluates stored budgets against actual spend.
pub struct BudgetEvaluator<'a> {
    snap: &'a Snapshot,
    month: Month,
}

impl<'a> BudgetEvaluator<'a> {
    /// Evaluator for `scope.month`.
    pub fn new(snap: &'a Snapshot, scope: &AuditScope) -> Self {
        Self { snap, month: scope.month }
    }

    /// Status per category with a budget effective this month.
    pub fn evaluate(&self) -> Vec<BudgetStatus> {
        let index = SpendIndex::build(self.snap);
        self.effective().into_values().map(|b| Self::status(b, index.spent(b.category, self.month))).collect()
    }

    fn effective(&self) -> BTreeMap<CategoryId, &'a Budget> {
        let mut chosen: BTreeMap<CategoryId, &'a Budget> = BTreeMap::new();
        for budget in self.snap.budgets.iter().filter(|b| b.covers(self.month)) {
            let slot = chosen.entry(budget.category).or_insert(budget);
            if budget.precedence() > slot.precedence() {
                *slot = budget;
            }
        }
        chosen
    }

    fn status(budget: &Budget, spent: Cents) -> BudgetStatus {
        let pct_used = match budget.limit.is_positive() {
            true => (spent.value() as f64 / budget.limit.value() as f64 * 1000.0).round() / 10.0,
            false => 0.0,
        };
        BudgetStatus { category: budget.category, limit: budget.limit, spent, remaining: budget.limit - spent, pct_used }
    }
}

/// Proposes limits from trailing spend.
pub struct BudgetProposer<'a> {
    snap: &'a Snapshot,
    month: Month,
}

impl<'a> BudgetProposer<'a> {
    /// Proposer for `scope.month`; the snapshot must cover the lookback window.
    pub fn new(snap: &'a Snapshot, scope: &AuditScope) -> Self {
        Self { snap, month: scope.month }
    }

    /// One proposal per expense category with spend in the lookback window.
    pub fn propose(&self) -> Vec<BudgetProposal> {
        let index = SpendIndex::build(self.snap);
        let months = self.lookback();
        let proposals = index.categories().map(|c| Self::proposal(&index, c, &months));
        proposals.filter(|p| p.proposed.is_positive()).collect()
    }

    fn lookback(&self) -> Vec<Month> {
        let mut months: Vec<Month> =
            std::iter::successors(Some(self.month.previous()), |m| Some(m.previous())).take(PROPOSAL_LOOKBACK_MONTHS as usize).collect();
        months.reverse();
        months
    }

    fn proposal(index: &SpendIndex, category: CategoryId, months: &[Month]) -> BudgetProposal {
        let history: Vec<Cents> = months.iter().map(|m| index.spent(category, *m)).collect();
        let sum = history.iter().copied().sum::<Cents>().value().max(0);
        let mean = Self::ceil_div(sum, months.len().max(1) as i64);
        let proposed = Cents::new(Self::ceil_div(mean, WHOLE_UNIT) * WHOLE_UNIT);
        BudgetProposal { category, history, proposed }
    }

    fn ceil_div(n: i64, d: i64) -> i64 {
        n.div_euclid(d) + i64::from(n.rem_euclid(d) > 0)
    }
}

/// Invariant set applied to budget proposals.
pub fn proposal_invariants() -> InvariantSet<Vec<BudgetProposal>> {
    InvariantSet::new(vec![Box::new(WholeUnitProposals)])
}

/// Every proposal is a positive whole-unit amount.
struct WholeUnitProposals;

impl Invariant<Vec<BudgetProposal>> for WholeUnitProposals {
    fn id(&self) -> &'static str {
        "whole_unit_proposals"
    }
    fn holds(&self, proposals: &Vec<BudgetProposal>, _: &Snapshot) -> bool {
        proposals.iter().all(|p| p.proposed.is_positive() && p.proposed.value() % WHOLE_UNIT == 0)
    }
}

/// Invariant set applied to budget status rows.
pub fn status_invariants() -> InvariantSet<Vec<BudgetStatus>> {
    InvariantSet::new(vec![Box::new(RemainingBalances)])
}

/// `remaining + spent == limit` on every row.
struct RemainingBalances;

impl Invariant<Vec<BudgetStatus>> for RemainingBalances {
    fn id(&self) -> &'static str {
        "remaining_balances"
    }
    fn holds(&self, rows: &Vec<BudgetStatus>, _: &Snapshot) -> bool {
        rows.iter().all(|r| r.remaining + r.spent == r.limit)
    }
}
