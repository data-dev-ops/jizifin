//! Accumulates paid and expected cents per bucket.

use super::BucketRow;
use crate::shares::{Bucket, Resolution};
use jizifin_domain::Cents;
use jizifin_domain::entities::Entry;
use jizifin_domain::ids::MemberId;
use std::collections::BTreeMap;

type MemberCents = BTreeMap<MemberId, Cents>;

/// Paid and expected cents per bucket and member.
#[derive(Default)]
pub(super) struct CategoryLedger {
    paid: BTreeMap<Bucket, MemberCents>,
    expected: BTreeMap<Bucket, MemberCents>,
}

impl CategoryLedger {
    /// Books one expense with its resolved shares.
    pub fn book(&mut self, entry: &Entry, payer: MemberId, resolution: &Resolution) {
        *self.paid.entry(resolution.bucket).or_default().entry(payer).or_default() += entry.amount;
        let expected = self.expected.entry(resolution.bucket).or_default();
        for (member, units) in resolution.allocate(entry).iter() {
            *expected.entry(member).or_default() += Cents::new(units);
        }
    }

    /// What `member` paid in `bucket`.
    pub fn paid_in(&self, bucket: Bucket, member: MemberId) -> Cents {
        self.paid.get(&bucket).and_then(|m| m.get(&member)).copied().unwrap_or_default()
    }

    /// Rows for every bucket that moves money. A bucket whose total nets to zero still counts when
    /// members' positions differ (one member paid, another received the refund).
    pub fn rows(&self) -> Vec<BucketRow> {
        self.paid.keys().map(|b| self.row(*b)).filter(Self::moves_money).collect()
    }

    /// Σ paid − expected per member across all buckets.
    pub fn net(&self) -> MemberCents {
        Self::net_of(&self.rows())
    }

    /// Σ of the row nets per member.
    pub fn net_of(rows: &[BucketRow]) -> MemberCents {
        let mut net = MemberCents::new();
        rows.iter().flat_map(|row| row.net.iter()).for_each(|(member, delta)| *net.entry(*member).or_default() += *delta);
        net
    }

    fn moves_money(row: &BucketRow) -> bool {
        !row.total.is_zero() || row.net.values().any(|c| !c.is_zero())
    }

    fn row(&self, bucket: Bucket) -> BucketRow {
        let paid = self.paid.get(&bucket).cloned().unwrap_or_default();
        let expected = self.expected.get(&bucket).cloned().unwrap_or_default();
        let total: Cents = paid.values().copied().sum();
        BucketRow {
            share_pct: Self::pct_of(&expected, total),
            paid_pct: Self::pct_of(&paid, total),
            net: Self::row_net(&paid, &expected),
            bucket,
            total,
            paid,
            expected,
        }
    }

    /// Each member's amount as a percentage of `total`, two decimals; empty when `total` is zero.
    fn pct_of(amounts: &MemberCents, total: Cents) -> BTreeMap<MemberId, f64> {
        let pct = |c: Cents| (c.value() as f64 / total.value() as f64 * 10_000.0).round() / 100.0;
        match total.is_zero() {
            true => BTreeMap::new(),
            false => amounts.iter().map(|(m, c)| (*m, pct(*c))).collect(),
        }
    }

    /// Paid − expected for every member present in either map.
    fn row_net(paid: &MemberCents, expected: &MemberCents) -> MemberCents {
        let members = paid.keys().chain(expected.keys());
        let amount = |map: &MemberCents, m: &MemberId| map.get(m).copied().unwrap_or_default();
        members.map(|m| (*m, amount(paid, m) - amount(expected, m))).collect()
    }
}
