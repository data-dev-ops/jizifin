//! Signed largest-remainder (Hare-Niemeyer) allocation with salted tie-breaks.

use jizifin_domain::ids::{EntryId, MemberId};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Relative weight of one member. Percentages, income cents and counts all work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weight {
    /// Member.
    pub member: MemberId,
    /// Non-negative weight.
    pub weight: f64,
}

/// Seed that makes tie-breaks deterministic per transaction instead of per name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TieSeed(String);

impl TieSeed {
    /// Seed for a ledger entry.
    pub fn entry(id: EntryId) -> Self {
        Self(id.to_string())
    }

    /// Seed for a named, non-entry calculation (e.g. `deposit:3:2026-10`).
    pub fn named(label: impl Into<String>) -> Self {
        Self(label.into())
    }

    fn rank_key(&self, member: MemberId) -> String {
        hex::encode(Sha256::digest(format!("{}:{}", self.0, member).as_bytes()))
    }
}

/// Integer units per member; the sum always equals the allocated total.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Allocation(BTreeMap<MemberId, i64>);

impl Allocation {
    /// Units of `member` (zero when absent).
    pub fn get(&self, member: MemberId) -> i64 {
        self.0.get(&member).copied().unwrap_or(0)
    }

    /// Sum of all units.
    pub fn total(&self) -> i64 {
        self.0.values().sum()
    }

    /// Iterates `(member, units)` in member order.
    pub fn iter(&self) -> impl Iterator<Item = (MemberId, i64)> + '_ {
        self.0.iter().map(|(m, u)| (*m, *u))
    }

    fn zeroed(weights: &[Weight]) -> Self {
        Self(weights.iter().map(|w| (w.member, 0)).collect())
    }
}

/// Divides an integer total across weights without losing a unit.
#[derive(Clone, Copy, Debug, Default)]
pub struct LargestRemainderAllocator;

impl LargestRemainderAllocator {
    /// Allocates `total` units (cents or points), preserving sign on refunds.
    pub fn allocate(&self, total: i64, weights: &[Weight], seed: &TieSeed) -> Allocation {
        let weight_sum: f64 = weights.iter().map(|w| w.weight.max(0.0)).sum();
        if total == 0 || weight_sum <= 0.0 {
            return Allocation::zeroed(weights);
        }
        let exact = Self::exact_units(total.abs(), weights, weight_sum);
        let mut floors = Self::floors(&exact);
        let leftover = total.abs() - floors.values().sum::<i64>();
        Self::distribute(&mut floors, &Self::ranked(&exact, seed), leftover);
        Allocation(floors.into_iter().map(|(m, u)| (m, u * total.signum())).collect())
    }

    fn exact_units(total: i64, weights: &[Weight], weight_sum: f64) -> Vec<(MemberId, f64)> {
        weights.iter().map(|w| (w.member, total as f64 * w.weight.max(0.0) / weight_sum)).collect()
    }

    fn floors(exact: &[(MemberId, f64)]) -> BTreeMap<MemberId, i64> {
        exact.iter().map(|(m, share)| (*m, share.floor() as i64)).collect()
    }

    fn ranked(exact: &[(MemberId, f64)], seed: &TieSeed) -> Vec<MemberId> {
        let mut keyed: Vec<(f64, String, MemberId)> =
            exact.iter().map(|(m, share)| (share - share.floor(), seed.rank_key(*m), *m)).collect();
        keyed.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        keyed.into_iter().map(|(_, _, m)| m).collect()
    }

    fn distribute(floors: &mut BTreeMap<MemberId, i64>, ranked: &[MemberId], leftover: i64) {
        let take = usize::try_from(leftover).unwrap_or(0);
        for member in ranked.iter().cycle().take(take) {
            *floors.entry(*member).or_default() += 1;
        }
    }
}
