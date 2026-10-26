//! Orchestrates ledger booking, offsets, partitioning and simplification.

use super::components::ComponentPartitioner;
use super::debts::DebtSimplifier;
use super::ledger::CategoryLedger;
use super::offsets::OffsetApplier;
use super::{Debt, Settlement};
use crate::income::IncomeResolver;
use crate::shares::{EntryShares, ShareResolver};
use crate::{AuditScope, Snapshot};
use jizifin_domain::Cents;
use jizifin_domain::entities::{Entry, EntryKind};
use jizifin_domain::ids::{EntryId, MemberId};
use std::collections::BTreeMap;

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
        let applied = OffsetApplier::new(&ledger).apply(&self.snap.offset_rules, &mut net);
        let rows = ledger.rows();
        let components = ComponentPartitioner::default().partition(&rows, &net, &applied);
        let debts: Vec<Debt> = components.iter().flat_map(|c| DebtSimplifier.simplify(c, &net)).collect();
        let transfer_total = debts.iter().map(|d| d.amount).sum();
        Settlement { month: self.scope.month, rows, net, components, debts, transfer_total }
    }

    /// One line per allocated share of every booked expense (`?explain=1`).
    pub fn explain(&self) -> Vec<String> {
        let resolver = self.resolver();
        let shares = self.payback_entries().filter_map(|e| Self::shares_of(&resolver, e));
        shares.flat_map(|s| s.trail().collect::<Vec<_>>()).collect()
    }

    /// Resolved split of one expense in the snapshot, if it is a payable expense.
    pub fn entry_shares(&self, id: EntryId) -> Option<EntryShares> {
        let entry = self.snap.entries.iter().find(|e| e.id == id && e.kind == EntryKind::Expense)?;
        Self::shares_of(&self.resolver(), entry)
    }

    /// Σ paid − Σ share per member, posted entry by entry without the bucket ledger.
    pub(crate) fn member_positions(&self) -> BTreeMap<MemberId, Cents> {
        let resolver = self.resolver();
        let mut positions = BTreeMap::new();
        self.payback_entries()
            .filter_map(|e| Some((e.member?, Self::shares_of(&resolver, e)?)))
            .for_each(|(payer, shares)| Self::post(&mut positions, payer, &shares));
        positions
    }

    fn post(positions: &mut BTreeMap<MemberId, Cents>, payer: MemberId, shares: &EntryShares) {
        *positions.entry(payer).or_default() += shares.total;
        shares.shares.iter().for_each(|(member, cents)| *positions.entry(*member).or_default() -= *cents);
    }

    fn shares_of(resolver: &ShareResolver, entry: &Entry) -> Option<EntryShares> {
        let (payer, category) = (entry.member?, entry.category?);
        Some(EntryShares::new(entry, &resolver.resolve(entry, payer, category)))
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
            .filter(|e| !e.is_joint_expense())
            .filter(move |e| Self::in_member_filter(snap, scope, e))
    }

    fn in_member_filter(snap: &Snapshot, scope: &AuditScope, entry: &Entry) -> bool {
        let Some(filter) = &scope.members else { return true };
        let payer_in = entry.member.is_some_and(|m| filter.contains(&m));
        let override_in = snap.overrides.get(&entry.id).is_some_and(|s| s.iter().any(|x| filter.contains(&x.member)));
        payer_in || override_in
    }
}
