//! Monthly cash flow of the scoped members: effective income against spending.

use crate::income::IncomeResolver;
use crate::invariant::{Invariant, InvariantSet};
use crate::{AuditScope, Snapshot};
use jizifin_domain::entities::EntryKind;
use jizifin_domain::ids::MemberId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;

/// Income, spending and what is left.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CashFlow {
    /// Month evaluated.
    pub month: Month,
    /// Σ effective income (contract or override, plus one-off income).
    pub income: Cents,
    /// Σ expenses paid.
    pub spent: Cents,
    /// `income − spent`.
    pub net: Cents,
    /// `net / income × 100`; absent without income.
    pub savings_rate_pct: Option<f64>,
}

/// Computes [`CashFlow`] for a scope.
pub struct CashFlowCalculator<'a> {
    snap: &'a Snapshot,
    scope: &'a AuditScope,
}

impl<'a> CashFlowCalculator<'a> {
    /// Calculator for `scope`.
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope }
    }

    /// Cash flow of the scoped members (everyone when unscoped).
    pub fn calculate(&self) -> CashFlow {
        let income = self.income();
        let spent = self.spent();
        let net = income - spent;
        let savings_rate_pct = income.is_positive().then(|| net.value() as f64 / income.value() as f64 * 100.0);
        CashFlow { month: self.scope.month, income, spent, net, savings_rate_pct }
    }

    fn in_scope(&self, member: MemberId) -> bool {
        self.scope.members.as_ref().is_none_or(|f| f.contains(&member))
    }

    fn income(&self) -> Cents {
        let resolved = IncomeResolver::new(self.snap, self.scope.month).resolve();
        resolved.values().filter(|i| self.in_scope(i.member)).map(|i| i.total).sum()
    }

    fn spent(&self) -> Cents {
        let expenses = self.snap.entries_in(EntryKind::Expense, self.scope.month);
        expenses.filter(|e| e.member.is_some_and(|m| self.in_scope(m))).map(|e| e.amount).sum()
    }
}

/// Invariant set applied to [`CashFlow`].
pub fn cash_flow_invariants() -> InvariantSet<CashFlow> {
    InvariantSet::new(vec![Box::new(NetBalances)])
}

/// `net` is exactly income minus spending.
struct NetBalances;

impl Invariant<CashFlow> for NetBalances {
    fn id(&self) -> &'static str {
        "net_balances"
    }
    fn holds(&self, c: &CashFlow, _: &Snapshot) -> bool {
        c.net == c.income - c.spent
    }
}
