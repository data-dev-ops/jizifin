//! Joint-account dashboard: balance, actual vs expected, deposit status.

use crate::deposit::DepositProposer;
use crate::invariant::{Invariant, InvariantSet};
use crate::{AuditScope, Snapshot};
use chrono::Datelike;
use jizifin_domain::entities::{Account, AccountMember, Entry, EntryKind};
use jizifin_domain::ids::{AccountId, CategoryId, MemberId};
use jizifin_domain::{Cents, Month};
use serde::Serialize;

/// Deposit progress for the month.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum DepositStatus {
    /// Deposits reached the target.
    TargetMet,
    /// Below target but deposits are still due later this month.
    Pending {
        /// Earliest remaining due day.
        due_day: u32,
    },
    /// Below target with nothing left due.
    BelowTarget,
}

/// Actual vs expected for one category.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CategoryUsage {
    /// Category.
    pub category: CategoryId,
    /// Spent this month.
    pub actual: Cents,
    /// Expected monthly cost.
    pub expected: Cents,
    /// `actual / expected` in percent, one decimal.
    pub pct_used: f64,
}

/// Deposit state of one member.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MemberDeposit {
    /// Member.
    pub member: MemberId,
    /// Deposited this month.
    pub deposited: Cents,
    /// Effective due day this month.
    pub due_day: u32,
}

/// Full dashboard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JointDashboard {
    /// Account.
    pub account: AccountId,
    /// Month.
    pub month: Month,
    /// Balance at month end: deposits + corrections − expenses, all time.
    pub balance: Cents,
    /// Σ joint expenses this month.
    pub actual_total: Cents,
    /// Deposits plus positive corrections this month.
    pub deposited_total: Cents,
    /// Expected monthly costs (override or Σ category expectations).
    pub expected_total: Cents,
    /// Safety margin applied to the expected costs.
    pub safety_margin_pct: f64,
    /// Deposit target.
    pub target: Cents,
    /// Deposit still needed to bring the balance up to the target.
    pub top_up: Cents,
    /// `deposited_total − actual_total` for the month (negative = deficit).
    pub month_surplus: Cents,
    /// Per-category usage.
    pub categories: Vec<CategoryUsage>,
    /// Per-member deposits.
    pub members: Vec<MemberDeposit>,
    /// Status.
    pub status: DepositStatus,
}

/// Computes [`JointDashboard`].
pub struct JointBalanceCalculator<'a> {
    snap: &'a Snapshot,
    scope: &'a AuditScope,
    account: &'a Account,
}

impl<'a> JointBalanceCalculator<'a> {
    /// Calculator for `account`.
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope, account: &'a Account) -> Self {
        Self { snap, scope, account }
    }

    /// Builds the dashboard.
    pub fn calculate(&self) -> JointDashboard {
        let target = DepositProposer::target(self.account);
        let deposited_total = self.deposited_total();
        let (balance, actual_total) = (self.balance(), self.month_spent());
        JointDashboard {
            account: self.account.id,
            month: self.scope.month,
            balance,
            actual_total,
            deposited_total,
            month_surplus: deposited_total - actual_total,
            expected_total: self.account.expected_total(),
            safety_margin_pct: self.account.safety_margin_pct,
            target,
            top_up: (target - balance).max(Cents::ZERO),
            categories: self.account.categories.iter().map(|c| self.usage(c.category, c.expected)).collect(),
            members: self.account.members.iter().map(|m| self.member_deposit(m)).collect(),
            status: self.status(deposited_total, target),
        }
    }

    fn account_entries(&self) -> impl Iterator<Item = &'a Entry> {
        let id = self.account.id;
        self.snap.entries.iter().filter(move |e| e.account == Some(id))
    }

    fn month_entries(&self, kind: EntryKind) -> impl Iterator<Item = &'a Entry> {
        let period = self.scope.month.period();
        self.account_entries().filter(move |e| e.kind == kind && period.contains(e.date))
    }

    fn month_spent(&self) -> Cents {
        self.month_entries(EntryKind::Expense).map(|e| e.amount).sum()
    }

    fn balance(&self) -> Cents {
        self.account_entries().map(Self::signed).sum()
    }

    fn signed(entry: &Entry) -> Cents {
        match entry.kind {
            EntryKind::Expense => -entry.amount,
            EntryKind::Deposit | EntryKind::Correction | EntryKind::Income => entry.amount,
        }
    }

    fn deposited_total(&self) -> Cents {
        let deposits: Cents = self.month_entries(EntryKind::Deposit).map(|e| e.amount).sum();
        let corrections = self.month_entries(EntryKind::Correction).map(|e| e.amount).filter(|c| c.is_positive());
        deposits + corrections.sum()
    }

    fn usage(&self, category: CategoryId, expected: Cents) -> CategoryUsage {
        let actual = self.month_entries(EntryKind::Expense).filter(|e| e.category == Some(category)).map(|e| e.amount).sum();
        CategoryUsage { category, actual, expected, pct_used: Self::pct(actual, expected) }
    }

    fn pct(actual: Cents, expected: Cents) -> f64 {
        match expected.is_positive() {
            true => (actual.value() as f64 / expected.value() as f64 * 1000.0).round() / 10.0,
            false => 0.0,
        }
    }

    fn member_deposit(&self, m: &AccountMember) -> MemberDeposit {
        let deposited = self.month_entries(EntryKind::Deposit).filter(|e| e.member == Some(m.member)).map(|e| e.amount).sum();
        MemberDeposit { member: m.member, deposited, due_day: self.scope.month.clamp_day(m.due_day) }
    }

    fn status(&self, deposited: Cents, target: Cents) -> DepositStatus {
        if deposited >= target {
            return DepositStatus::TargetMet;
        }
        self.next_due_day().map_or(DepositStatus::BelowTarget, |due_day| DepositStatus::Pending { due_day })
    }

    fn next_due_day(&self) -> Option<u32> {
        let current = Month::of(self.scope.as_of) == self.scope.month;
        let today = self.scope.as_of.day();
        let unpaid = self.account.members.iter().map(|m| self.member_deposit(m)).filter(|d| d.deposited.is_zero());
        unpaid.map(|d| d.due_day).filter(|day| current && *day > today).min()
    }
}

/// Invariant set applied to [`JointDashboard`].
pub fn joint_invariants() -> InvariantSet<JointDashboard> {
    InvariantSet::new(vec![Box::new(CategoriesWithinActual), Box::new(TargetCoversExpected), Box::new(TopUpClosesGap)])
}

/// Σ category actuals never exceeds the month's joint expense total.
struct CategoriesWithinActual;

impl Invariant<JointDashboard> for CategoriesWithinActual {
    fn id(&self) -> &'static str {
        "categories_within_actual"
    }
    fn holds(&self, d: &JointDashboard, _: &Snapshot) -> bool {
        d.categories.iter().map(|c| c.actual).sum::<Cents>().abs() <= d.actual_total.abs()
    }
}

/// With a non-negative margin the target covers the expected costs.
struct TargetCoversExpected;

impl Invariant<JointDashboard> for TargetCoversExpected {
    fn id(&self) -> &'static str {
        "target_covers_expected"
    }
    fn holds(&self, d: &JointDashboard, snap: &Snapshot) -> bool {
        let Ok(account) = snap.account(d.account) else { return false };
        account.safety_margin_pct < 0.0 || d.target >= account.expected_total()
    }
}

/// Balance plus the proposed top-up reaches the target, and the top-up is never negative.
struct TopUpClosesGap;

impl Invariant<JointDashboard> for TopUpClosesGap {
    fn id(&self) -> &'static str {
        "top_up_closes_gap"
    }
    fn holds(&self, d: &JointDashboard, _: &Snapshot) -> bool {
        !d.top_up.is_negative() && d.balance + d.top_up >= d.target
    }
}
