//! Budget usage and proposals.

use crate::invariant::{Invariant, InvariantSet};
use crate::{AuditScope, Snapshot};
use jizifin_domain::entities::{Budget, Entry, EntryKind};
use jizifin_domain::ids::CategoryId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

/// Months of history a proposal averages over.
pub const PROPOSAL_LOOKBACK_MONTHS: u32 = 3;

/// Proposals are rounded up to whole currency units.
const WHOLE_UNIT: i64 = 100;

/// Usage below this percentage counts as safe in the health index.
const SAFE_PCT_USED: f64 = 90.0;

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

/// Totals across every budgeted category.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BudgetSummary {
    /// Σ limits.
    pub total_limit: Cents,
    /// Σ spent in budgeted categories.
    pub total_spent: Cents,
    /// `max(0, total_limit − total_spent)`.
    pub total_remaining: Cents,
    /// `total_spent / total_limit` in percent; absent without limits.
    pub usage_pct: Option<f64>,
    /// Categories with a positive limit.
    pub configured: usize,
    /// Of those, categories below 90 % usage.
    pub safe: usize,
    /// `safe / configured` in percent (100 when nothing is configured).
    pub health_pct: f64,
}

/// Status rows plus their totals.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BudgetReport {
    /// One row per budgeted category.
    pub rows: Vec<BudgetStatus>,
    /// Totals.
    pub summary: BudgetSummary,
}

impl BudgetSummary {
    fn of(rows: &[BudgetStatus]) -> Self {
        let total_limit: Cents = rows.iter().map(|r| r.limit).sum();
        let total_spent: Cents = rows.iter().map(|r| r.spent).sum();
        let configured = rows.iter().filter(|r| r.limit.is_positive()).count();
        let safe = rows.iter().filter(|r| r.limit.is_positive() && r.pct_used < SAFE_PCT_USED).count();
        Self {
            total_limit,
            total_spent,
            total_remaining: (total_limit - total_spent).max(Cents::ZERO),
            usage_pct: total_limit.is_positive().then(|| total_spent.value() as f64 / total_limit.value() as f64 * 100.0),
            configured,
            safe,
            health_pct: if configured == 0 { 100.0 } else { safe as f64 / configured as f64 * 100.0 },
        }
    }
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
    /// Expense spend in the scope: paid by the scoped members and, with an account, charged to it.
    fn build(snap: &Snapshot, scope: &AuditScope) -> Self {
        let mut index = BTreeMap::new();
        let in_scope = |e: &&Entry| {
            let member_ok = scope.members.as_ref().is_none_or(|f| e.member.is_some_and(|m| f.contains(&m)));
            e.kind == EntryKind::Expense && member_ok && scope.account.is_none_or(|a| e.account == Some(a))
        };
        let expenses = snap.entries.iter().filter(in_scope);
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
    scope: &'a AuditScope,
    month: Month,
}

impl<'a> BudgetEvaluator<'a> {
    /// Evaluator for `scope` (month, optional members and account).
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope, month: scope.month }
    }

    /// Status per category with a budget effective this month, plus totals.
    pub fn evaluate(&self) -> BudgetReport {
        let index = SpendIndex::build(self.snap, self.scope);
        let rows: Vec<BudgetStatus> = self.effective().into_values().map(|b| Self::status(b, index.spent(b.category, self.month))).collect();
        BudgetReport { summary: BudgetSummary::of(&rows), rows }
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
    scope: &'a AuditScope,
    month: Month,
}

impl<'a> BudgetProposer<'a> {
    /// Proposer for `scope.month`; the snapshot must cover the lookback window.
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope, month: scope.month }
    }

    /// One proposal per expense category with spend in the lookback window.
    pub fn propose(&self) -> Vec<BudgetProposal> {
        let index = SpendIndex::build(self.snap, self.scope);
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

/// Invariant set applied to budget reports.
pub fn status_invariants() -> InvariantSet<BudgetReport> {
    InvariantSet::new(vec![Box::new(RemainingBalances), Box::new(SummaryMatchesRows)])
}

/// `remaining + spent == limit` on every row.
struct RemainingBalances;

impl Invariant<BudgetReport> for RemainingBalances {
    fn id(&self) -> &'static str {
        "remaining_balances"
    }
    fn holds(&self, r: &BudgetReport, _: &Snapshot) -> bool {
        r.rows.iter().all(|row| row.remaining + row.spent == row.limit)
    }
}

/// Summary totals equal the row sums.
struct SummaryMatchesRows;

impl Invariant<BudgetReport> for SummaryMatchesRows {
    fn id(&self) -> &'static str {
        "summary_matches_rows"
    }
    fn holds(&self, r: &BudgetReport, _: &Snapshot) -> bool {
        r.summary.total_limit == r.rows.iter().map(|row| row.limit).sum::<Cents>()
            && r.summary.total_spent == r.rows.iter().map(|row| row.spent).sum::<Cents>()
    }
}
