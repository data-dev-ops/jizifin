//! Monthly peer settlement: who owes whom, isolated per connected household group.

mod calculator;
mod components;
mod debts;
mod invariants;
mod ledger;
mod offsets;

pub use calculator::SettlementCalculator;
pub use invariants::settlement_invariants;
pub(crate) use debts::DebtSimplifier;
pub(crate) use invariants::debts_settle;

use crate::shares::Bucket;
use jizifin_domain::ids::MemberId;
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

/// Paid vs expected per member for one bucket.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BucketRow {
    /// Category and, for personal pay, the payer.
    pub bucket: Bucket,
    /// Σ paid in the bucket.
    pub total: Cents,
    /// Amount each member paid.
    pub paid: BTreeMap<MemberId, Cents>,
    /// Amount each member is liable for.
    pub expected: BTreeMap<MemberId, Cents>,
    /// Liability share in percent, two decimals, for display.
    pub share_pct: BTreeMap<MemberId, f64>,
    /// Share of the bucket total each member paid, two decimals, for display.
    pub paid_pct: BTreeMap<MemberId, f64>,
    /// Paid − expected per member (positive = owed money back).
    pub net: BTreeMap<MemberId, Cents>,
}

/// One simplified transfer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Debt {
    /// Debtor.
    pub from: MemberId,
    /// Creditor.
    pub to: MemberId,
    /// Transfer amount.
    pub amount: Cents,
}

/// Full settlement of a month.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Settlement {
    /// Month settled.
    pub month: Month,
    /// Per-bucket breakdown.
    pub rows: Vec<BucketRow>,
    /// Net position per member after offsets (positive = owed money).
    pub net: BTreeMap<MemberId, Cents>,
    /// Disjoint member groups debts are settled within.
    pub components: Vec<Vec<MemberId>>,
    /// Simplified transfers.
    pub debts: Vec<Debt>,
    /// Σ transfer amounts.
    pub transfer_total: Cents,
}
