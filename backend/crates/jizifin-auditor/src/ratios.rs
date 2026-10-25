//! Whole-percent split proposals derived from income.

use crate::allocation::{LargestRemainderAllocator, TieSeed};
use crate::income::IncomeResolver;
use crate::invariant::{Invariant, InvariantSet};
use crate::{AuditScope, Snapshot};
use jizifin_domain::Month;
use jizifin_domain::ids::MemberId;
use serde::Serialize;
use std::collections::BTreeMap;

/// Integer percentages summing to 100.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SalaryRatios {
    /// Month the incomes were taken from.
    pub month: Month,
    /// Percentage per member.
    pub pct: BTreeMap<MemberId, i64>,
}

/// Turns effective incomes into a proposed agreement.
pub struct SalaryRatioCalculator<'a> {
    snap: &'a Snapshot,
    scope: &'a AuditScope,
}

impl<'a> SalaryRatioCalculator<'a> {
    /// Calculator for `scope`.
    pub fn new(snap: &'a Snapshot, scope: &'a AuditScope) -> Self {
        Self { snap, scope }
    }

    /// Allocates 100 points by income; equal when nobody earns.
    pub fn calculate(&self) -> SalaryRatios {
        let mut weights = IncomeResolver::new(self.snap, self.scope.month).weights();
        weights.retain(|w| self.scope.members.as_ref().is_none_or(|f| f.contains(&w.member)));
        if weights.iter().all(|w| w.weight <= 0.0) {
            weights.iter_mut().for_each(|w| w.weight = 1.0);
        }
        let seed = TieSeed::named(format!("ratio:{}", self.scope.month));
        let points = LargestRemainderAllocator.allocate(100, &weights, &seed);
        SalaryRatios { month: self.scope.month, pct: points.iter().collect() }
    }
}

/// Invariant set applied to [`SalaryRatios`].
pub fn ratio_invariants() -> InvariantSet<SalaryRatios> {
    InvariantSet::new(vec![Box::new(RatiosSumTo100)])
}

/// Percentages sum to exactly 100 whenever any member is present.
struct RatiosSumTo100;

impl Invariant<SalaryRatios> for RatiosSumTo100 {
    fn id(&self) -> &'static str {
        "ratios_sum_to_100"
    }
    fn holds(&self, r: &SalaryRatios, _: &Snapshot) -> bool {
        r.pct.is_empty() || r.pct.values().sum::<i64>() == 100
    }
}
