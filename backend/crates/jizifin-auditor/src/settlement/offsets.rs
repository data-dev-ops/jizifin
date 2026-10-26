//! Applies configured netting rules to member net positions.

use super::ledger::CategoryLedger;
use crate::shares::Bucket;
use jizifin_domain::Cents;
use jizifin_domain::entities::OffsetRule;
use jizifin_domain::ids::{CategoryId, MemberId};
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

    /// Applies every active rule in order, returning member pairs that transferred funds.
    pub fn apply(&self, rules: &[OffsetRule], net: &mut BTreeMap<MemberId, Cents>) -> Vec<(MemberId, MemberId)> {
        rules.iter().filter(|rule| rule.is_active).filter_map(|rule| self.apply_rule(rule, net)).collect()
    }

    fn apply_rule(&self, rule: &OffsetRule, net: &mut BTreeMap<MemberId, Cents>) -> Option<(MemberId, MemberId)> {
        let amount = self.offset_amount(rule);
        if amount.is_positive() {
            *net.entry(rule.debit_member).or_default() -= amount;
            *net.entry(rule.credit_member).or_default() += amount;
            Some((rule.debit_member, rule.credit_member))
        } else {
            None
        }
    }

    fn offset_amount(&self, rule: &OffsetRule) -> Cents {
        let credit = self.paid(rule.credit_category, rule.credit_member);
        let debit = self.paid(rule.debit_category, rule.debit_member);
        match credit.is_positive() && debit.is_positive() {
            true => credit.min(debit),
            false => Cents::ZERO,
        }
    }

    /// What `member` paid in `category`, whether it is booked as a shared or as the member's personal bucket.
    fn paid(&self, category: CategoryId, member: MemberId) -> Cents {
        let shared = Bucket { category, personal_of: None };
        let personal = Bucket { category, personal_of: Some(member) };
        self.ledger.paid_in(shared, member) + self.ledger.paid_in(personal, member)
    }
}
