//! Orchestrates ledger booking, offsets, partitioning and simplification.

use super::components::ComponentPartitioner;
use super::debts::DebtSimplifier;
use super::ledger::CategoryLedger;
use super::offsets::OffsetApplier;
use super::Settlement;
use crate::income::IncomeResolver;
use crate::shares::ShareResolver;
use crate::{AuditScope, Snapshot};
use jizifin_domain::entities::{Entry, EntryKind};
use jizifin_domain::ids::MemberId;

/// Computes the peer settlement of a month.
pub struct SettlementCalculator<'a> {
    snap: &'a Snapshot,
    scope: &'a AuditScope,
}

impl<'a> SettlementCalculator<'a> {
    /// Calculator over `snap` for `scope`.
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope }
    }

    /// Runs the full settlement.
    pub fn calculate(&self) -> Settlement {
        let ledger = self.book_ledger();
        let mut net = ledger.net();
        OffsetApplier::new(&ledger).apply(&self.snap.offset_rules, &mut net);
        let rows = ledger.rows();
        let components = ComponentPartitioner::default().partition(&rows, &net);
        let debts = components.iter().flat_map(|c| DebtSimplifier.simplify(c, &net)).collect();
        Settlement { month: self.scope.month, rows, net, components, debts }
    }

    fn book_ledger(&self) -> CategoryLedger {
        let resolver = self.resolver();
        let mut ledger = CategoryLedger::default();
        for entry in self.payback_entries() {
            if let (Some(payer), Some(category)) = (entry.member, entry.category) {
                ledger.book(entry, payer, &resolver.resolve(entry, payer, category));
            }
        }
        ledger
    }

    fn resolver(&self) -> ShareResolver<'a> {
        let weights = IncomeResolver::new(self.snap, self.scope.month).weights();
        ShareResolver::new(self.snap, weights, self.participants())
    }

    fn participants(&self) -> Vec<MemberId> {
        match &self.scope.members {
            Some(filter) => filter.iter().copied().collect(),
            None => self.snap.active_members(),
        }
    }

    fn payback_entries(&self) -> impl Iterator<Item = &'a Entry> {
        let (snap, scope) = (self.snap, self.scope);
        snap.entries_in(EntryKind::Expense, scope.month)
            .filter(move |e| !e.is_joint_expense() && !e.category.is_some_and(|c| snap.is_joint_category(c)))
            .filter(move |e| Self::in_member_filter(snap, scope, e))
    }

    fn in_member_filter(snap: &Snapshot, scope: &AuditScope, entry: &Entry) -> bool {
        let Some(filter) = &scope.members else { return true };
        let payer_in = entry.member.is_some_and(|m| filter.contains(&m));
        let override_in = snap.overrides.get(&entry.id).is_some_and(|s| s.iter().any(|x| filter.contains(&x.member)));
        payer_in || override_in
    }
}
