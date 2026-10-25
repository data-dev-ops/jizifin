//! Applies configured netting rules to member net positions.

use super::ledger::CategoryLedger;
use crate::shares::Bucket;
use jizifin_domain::Cents;
use jizifin_domain::entities::OffsetRule;
use jizifin_domain::ids::MemberId;
use std::collections::BTreeMap;

/// Moves `min(credit paid, debit paid)` from the debit member to the credit member.
pub(super) struct OffsetApplier<'a> {
    ledger: &'a CategoryLedger,
}

impl<'a> OffsetApplier<'a> {
    /// Applier reading paid amounts from `ledger`.
    pub fn new(ledger: &'a CategoryLedger) -> Self {
        Self { ledger }
    }

    /// Applies every rule in order.
    pub fn apply(&self, rules: &[OffsetRule], net: &mut BTreeMap<MemberId, Cents>) {
        rules.iter().for_each(|rule| self.apply_rule(rule, net));
    }

    fn apply_rule(&self, rule: &OffsetRule, net: &mut BTreeMap<MemberId, Cents>) {
        let amount = self.offset_amount(rule);
        if amount.is_positive() {
            *net.entry(rule.debit_member).or_default() -= amount;
            *net.entry(rule.credit_member).or_default() += amount;
        }
    }

    fn offset_amount(&self, rule: &OffsetRule) -> Cents {
        let credit = self.ledger.paid_in(Self::shared(rule.credit_category), rule.credit_member);
        let debit = self.ledger.paid_in(Self::shared(rule.debit_category), rule.debit_member);
        match credit.is_positive() && debit.is_positive() {
            true => credit.min(debit),
            false => Cents::ZERO,
        }
    }

    fn shared(category: jizifin_domain::ids::CategoryId) -> Bucket {
        Bucket { category, personal_of: None }
    }
}
