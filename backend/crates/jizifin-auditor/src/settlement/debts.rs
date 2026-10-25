//! Greedy debt simplification inside one component.

use super::Debt;
use jizifin_domain::Cents;
use jizifin_domain::ids::MemberId;
use std::collections::BTreeMap;

type Ledger = Vec<(MemberId, Cents)>;

/// Matches the largest creditor with the largest debtor until all nets are zero.
#[derive(Default)]
pub(super) struct DebtSimplifier;

impl DebtSimplifier {
    /// Transfers settling every member of `component`.
    pub fn simplify(&self, component: &[MemberId], net: &BTreeMap<MemberId, Cents>) -> Vec<Debt> {
        let creditors = Self::side(component, net, Cents::is_positive, |c| c);
        let debtors = Self::side(component, net, Cents::is_negative, |c| -c);
        Self::match_sides(creditors, debtors)
    }

    fn side(
        component: &[MemberId],
        net: &BTreeMap<MemberId, Cents>,
        keep: fn(Cents) -> bool,
        magnitude: fn(Cents) -> Cents,
    ) -> Ledger {
        let mut side: Ledger =
            component.iter().map(|m| (*m, net[m])).filter(|(_, c)| keep(*c)).map(|(m, c)| (m, magnitude(c))).collect();
        side.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        side
    }

    fn match_sides(mut creditors: Ledger, mut debtors: Ledger) -> Vec<Debt> {
        let (mut ci, mut di, mut debts) = (0, 0, Vec::new());
        while ci < creditors.len() && di < debtors.len() {
            let amount = creditors[ci].1.min(debtors[di].1);
            debts.push(Debt { from: debtors[di].0, to: creditors[ci].0, amount });
            creditors[ci].1 -= amount;
            debtors[di].1 -= amount;
            ci += usize::from(creditors[ci].1.is_zero());
            di += usize::from(debtors[di].1.is_zero());
        }
        debts
    }
}
