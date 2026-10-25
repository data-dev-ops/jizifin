//! Invariants every settlement must satisfy.

use super::Settlement;
use crate::Snapshot;
use crate::invariant::{Invariant, InvariantSet};
use jizifin_domain::Cents;
use jizifin_domain::ids::MemberId;
use std::collections::BTreeMap;

/// Invariant set applied to [`Settlement`].
pub fn settlement_invariants() -> InvariantSet<Settlement> {
    InvariantSet::new(vec![
        Box::new(BucketZeroSum),
        Box::new(NetZeroSum),
        Box::new(DebtsSettleNet),
        Box::new(DebtsStayInComponent),
        Box::new(JointExcluded),
    ])
}

/// Σ expected equals the bucket total in every row.
struct BucketZeroSum;

impl Invariant<Settlement> for BucketZeroSum {
    fn id(&self) -> &'static str {
        "bucket_zero_sum"
    }
    fn holds(&self, s: &Settlement, _: &Snapshot) -> bool {
        s.rows.iter().all(|r| r.expected.values().copied().sum::<Cents>() == r.total)
    }
}

/// Net positions sum to zero across the household.
struct NetZeroSum;

impl Invariant<Settlement> for NetZeroSum {
    fn id(&self) -> &'static str {
        "net_zero_sum"
    }
    fn holds(&self, s: &Settlement, _: &Snapshot) -> bool {
        s.net.values().copied().sum::<Cents>().is_zero()
    }
}

/// Applying every debt brings every net position to zero.
struct DebtsSettleNet;

impl Invariant<Settlement> for DebtsSettleNet {
    fn id(&self) -> &'static str {
        "debts_settle_net"
    }
    fn holds(&self, s: &Settlement, _: &Snapshot) -> bool {
        debts_settle(&s.net, &s.debts)
    }
}

/// True when applying `debts` zeroes every position in `net`.
pub(crate) fn debts_settle(net: &BTreeMap<MemberId, Cents>, debts: &[super::Debt]) -> bool {
    let mut rest = net.clone();
    for d in debts {
        *rest.entry(d.from).or_default() += d.amount;
        *rest.entry(d.to).or_default() -= d.amount;
    }
    rest.values().all(|c| c.is_zero())
}

/// No transfer crosses component boundaries.
struct DebtsStayInComponent;

impl Invariant<Settlement> for DebtsStayInComponent {
    fn id(&self) -> &'static str {
        "debts_stay_in_component"
    }
    fn holds(&self, s: &Settlement, _: &Snapshot) -> bool {
        s.debts.iter().all(|d| s.components.iter().any(|c| c.contains(&d.from) && c.contains(&d.to)))
    }
}

/// No joint-account category appears in peer settlement.
struct JointExcluded;

impl Invariant<Settlement> for JointExcluded {
    fn id(&self) -> &'static str {
        "joint_excluded"
    }
    fn holds(&self, s: &Settlement, snap: &Snapshot) -> bool {
        s.rows.iter().all(|r| !snap.is_joint_category(r.bucket.category))
    }
}
