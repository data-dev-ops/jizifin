//! Proposed joint-account deposits per member.

use crate::allocation::{LargestRemainderAllocator, TieSeed, Weight};
use crate::income::IncomeResolver;
use crate::invariant::{Invariant, InvariantSet};
use crate::Snapshot;
use jizifin_domain::entities::{Account, DepositMode};
use jizifin_domain::ids::{AccountId, MemberId};
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

/// What each member should deposit this month.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DepositProposal {
    /// Account.
    pub account: AccountId,
    /// Month.
    pub month: Month,
    /// Division mode used.
    pub mode: DepositMode,
    /// Σ expected category costs.
    pub expected_total: Cents,
    /// Safety margin applied.
    pub safety_margin_pct: f64,
    /// `expected_total × (1 + margin)`, ties to even.
    pub target: Cents,
    /// Proposed amount per member.
    pub per_member: BTreeMap<MemberId, Cents>,
}

/// Computes [`DepositProposal`] for one account.
pub struct DepositProposer<'a> {
    snap: &'a Snapshot,
    account: &'a Account,
    month: Month,
}

impl<'a> DepositProposer<'a> {
    /// Proposer for `account` in `month`.
    pub fn new(snap: &'a Snapshot, account: &'a Account, month: Month) -> Self {
        Self { snap, account, month }
    }

    /// Target for the month.
    pub fn target(account: &Account) -> Cents {
        let scaled = account.expected_total().value() as f64 * (1.0 + account.safety_margin_pct / 100.0);
        Cents::new(scaled.round_ties_even() as i64)
    }

    /// Builds the proposal.
    pub fn propose(&self) -> DepositProposal {
        let target = Self::target(self.account);
        DepositProposal {
            account: self.account.id,
            month: self.month,
            mode: self.account.deposit_mode,
            expected_total: self.account.expected_total(),
            safety_margin_pct: self.account.safety_margin_pct,
            target,
            per_member: self.divide(target),
        }
    }

    fn divide(&self, target: Cents) -> BTreeMap<MemberId, Cents> {
        match self.account.deposit_mode {
            DepositMode::Fixed => self.fixed(),
            DepositMode::Equal => self.allocate(target, self.equal_weights()),
            DepositMode::SalaryRatio => self.allocate(target, self.salary_weights()),
        }
    }

    fn fixed(&self) -> BTreeMap<MemberId, Cents> {
        self.account.members.iter().map(|m| (m.member, m.fixed_deposit.unwrap_or_default())).collect()
    }

    fn allocate(&self, target: Cents, weights: Vec<Weight>) -> BTreeMap<MemberId, Cents> {
        let seed = TieSeed::named(format!("deposit:{}:{}", self.account.id, self.month));
        let alloc = LargestRemainderAllocator.allocate(target.value(), &weights, &seed);
        alloc.iter().map(|(m, u)| (m, Cents::new(u))).collect()
    }

    fn equal_weights(&self) -> Vec<Weight> {
        self.account.members.iter().map(|m| Weight { member: m.member, weight: 1.0 }).collect()
    }

    fn salary_weights(&self) -> Vec<Weight> {
        let incomes = IncomeResolver::new(self.snap, self.month).weights();
        let weights: Vec<Weight> = incomes.into_iter().filter(|w| self.is_member(w.member)).collect();
        match weights.iter().any(|w| w.weight > 0.0) {
            true => weights,
            false => self.equal_weights(),
        }
    }

    fn is_member(&self, member: MemberId) -> bool {
        self.account.members.iter().any(|m| m.member == member)
    }
}

/// Invariant set applied to [`DepositProposal`].
pub fn deposit_invariants() -> InvariantSet<DepositProposal> {
    InvariantSet::new(vec![Box::new(DepositSumsToTarget), Box::new(DepositNonNegative)])
}

/// Proposed deposits add up to the target (fixed mode is user-defined and exempt).
struct DepositSumsToTarget;

impl Invariant<DepositProposal> for DepositSumsToTarget {
    fn id(&self) -> &'static str {
        "deposit_sums_to_target"
    }
    fn holds(&self, p: &DepositProposal, _: &Snapshot) -> bool {
        p.mode == DepositMode::Fixed || p.per_member.values().copied().sum::<Cents>() == p.target
    }
}

/// No member is asked for a negative deposit.
struct DepositNonNegative;

impl Invariant<DepositProposal> for DepositNonNegative {
    fn id(&self) -> &'static str {
        "deposit_non_negative"
    }
    fn holds(&self, p: &DepositProposal, _: &Snapshot) -> bool {
        p.per_member.values().all(|c| !c.is_negative())
    }
}
