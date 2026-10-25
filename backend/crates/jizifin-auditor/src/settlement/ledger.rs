//! Accumulates paid and expected cents per bucket.

use super::BucketRow;
use crate::allocation::{LargestRemainderAllocator, TieSeed};
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
    allocator: LargestRemainderAllocator,
}

impl CategoryLedger {
    /// Books one expense with its resolved shares.
    pub fn book(&mut self, entry: &Entry, payer: MemberId, resolution: &Resolution) {
        *self.paid.entry(resolution.bucket).or_default().entry(payer).or_default() += entry.amount;
        let alloc = self.allocator.allocate(entry.amount.value(), &resolution.weights, &TieSeed::entry(entry.id));
        let expected = self.expected.entry(resolution.bucket).or_default();
        for (member, units) in alloc.iter() {
            *expected.entry(member).or_default() += Cents::new(units);
        }
    }

    /// What `member` paid in `bucket`.
    pub fn paid_in(&self, bucket: Bucket, member: MemberId) -> Cents {
        self.paid.get(&bucket).and_then(|m| m.get(&member)).copied().unwrap_or_default()
    }

    /// Rows for every bucket with a non-zero total.
    pub fn rows(&self) -> Vec<BucketRow> {
        self.paid.keys().filter_map(|b| self.row(*b)).collect()
    }

    /// Σ paid − expected per member across all buckets.
    pub fn net(&self) -> MemberCents {
        let mut net = MemberCents::new();
        self.rows().iter().for_each(|row| Self::add_row_net(&mut net, row));
        net
    }

    fn add_row_net(net: &mut MemberCents, row: &BucketRow) {
        for member in Self::row_members(row) {
            let delta = row.paid.get(&member).copied().unwrap_or_default()
                - row.expected.get(&member).copied().unwrap_or_default();
            *net.entry(member).or_default() += delta;
        }
    }

    fn row(&self, bucket: Bucket) -> Option<BucketRow> {
        let paid = self.paid.get(&bucket).cloned().unwrap_or_default();
        let expected = self.expected.get(&bucket).cloned().unwrap_or_default();
        let total: Cents = paid.values().copied().sum();
        let share_pct = Self::share_pct(&expected, total);
        (!total.is_zero()).then_some(BucketRow { bucket, total, paid, expected, share_pct })
    }

    fn share_pct(expected: &MemberCents, total: Cents) -> BTreeMap<MemberId, f64> {
        let pct = |c: Cents| (c.value() as f64 / total.value() as f64 * 10_000.0).round() / 100.0;
        expected.iter().map(|(m, c)| (*m, pct(*c))).collect()
    }

    fn row_members(row: &BucketRow) -> impl Iterator<Item = MemberId> + '_ {
        row.paid.keys().chain(row.expected.keys().filter(|m| !row.paid.contains_key(m))).copied()
    }
}
