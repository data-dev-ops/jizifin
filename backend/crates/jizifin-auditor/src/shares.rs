//! Resolves who bears which share of an expense.
//! Precedence: personal pay → entry override → income ratio → SCD2 agreement → equal split.

use crate::Snapshot;
use crate::allocation::{Allocation, LargestRemainderAllocator, TieSeed, Weight};
use jizifin_domain::Cents;
use jizifin_domain::entities::{Entry, PayMode, Share};
use jizifin_domain::ids::{CategoryId, EntryId, MemberId};
use serde::Serialize;
use std::collections::BTreeMap;

/// Settlement row key. Personal-pay expenses get one bucket per payer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Bucket {
    /// Category.
    pub category: CategoryId,
    /// Payer, for personal-pay categories only.
    pub personal_of: Option<MemberId>,
}

/// Which rule produced the shares (reported for explainability).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShareSource {
    /// Personal-pay category.
    Personal,
    /// Entry-level override.
    Override,
    /// Income-ratio category.
    IncomeRatio,
    /// Split agreement.
    Agreement,
    /// Equal fallback.
    Equal,
}

/// Resolved liability for one expense.
#[derive(Clone, Debug)]
pub struct Resolution {
    /// Settlement row.
    pub bucket: Bucket,
    /// Weights to allocate the amount with.
    pub weights: Vec<Weight>,
    /// Rule that applied.
    pub source: ShareSource,
}

impl ShareSource {
    /// Stable label used in explanation trails.
    pub fn label(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Override => "override",
            Self::IncomeRatio => "income_ratio",
            Self::Agreement => "agreement",
            Self::Equal => "equal",
        }
    }
}

impl Resolution {
    /// Splits the entry amount over this resolution's weights, seeded by the entry id.
    pub fn allocate(&self, entry: &Entry) -> Allocation {
        LargestRemainderAllocator.allocate(entry.amount.value(), &self.weights, &TieSeed::entry(entry.id))
    }
}

/// Who bears what for one expense.
#[derive(Clone, Debug, Serialize)]
pub struct EntryShares {
    /// Expense the shares belong to.
    pub entry: EntryId,
    /// Rule that applied.
    pub source: ShareSource,
    /// The expense amount.
    pub total: Cents,
    /// Liability per member; sums to `total`.
    pub shares: BTreeMap<MemberId, Cents>,
}

impl EntryShares {
    /// Allocates `entry` through `resolution`.
    pub fn new(entry: &Entry, resolution: &Resolution) -> Self {
        let shares = resolution.allocate(entry).iter().map(|(m, units)| (m, Cents::new(units))).collect();
        Self { entry: entry.id, source: resolution.source, total: entry.amount, shares }
    }

    /// Σ of the member shares.
    pub fn shares_total(&self) -> Cents {
        self.shares.values().copied().sum()
    }

    /// One explanation line per member share.
    pub fn trail(&self) -> impl Iterator<Item = String> + '_ {
        self.shares.iter().map(|(member, cents)| {
            format!("share(entry {}, member {}) = {} via {}", self.entry, member, cents.value(), self.source.label())
        })
    }
}

/// Applies the share precedence chain for one snapshot.
pub struct ShareResolver<'a> {
    snap: &'a Snapshot,
    income_weights: Vec<Weight>,
    participants: Vec<MemberId>,
}

impl<'a> ShareResolver<'a> {
    /// Resolver with precomputed income weights and equal-split participants.
    pub fn new(snap: &'a Snapshot, income_weights: Vec<Weight>, participants: Vec<MemberId>) -> Self {
        Self { snap, income_weights, participants }
    }

    /// Resolves the shares of an expense that has a payer and a category.
    pub fn resolve(&self, entry: &Entry, payer: MemberId, category: CategoryId) -> Resolution {
        let mode = self.snap.category(category).map_or(PayMode::Shared, |c| c.pay_mode);
        let shared = Bucket { category, personal_of: None };
        match mode {
            PayMode::Personal => Self::personal(category, payer),
            _ => self.shared(entry, mode, shared),
        }
    }

    /// Resolves the shares of an expense nobody paid personally (e.g. a joint-account expense): personal pay mode is treated as shared.
    pub fn resolve_unpaid(&self, entry: &Entry, category: CategoryId) -> Resolution {
        let mode = self.snap.category(category).map_or(PayMode::Shared, |c| c.pay_mode);
        self.shared(entry, mode, Bucket { category, personal_of: None })
    }

    fn personal(category: CategoryId, payer: MemberId) -> Resolution {
        let bucket = Bucket { category, personal_of: Some(payer) };
        Resolution { bucket, weights: vec![Weight { member: payer, weight: 100.0 }], source: ShareSource::Personal }
    }

    fn shared(&self, entry: &Entry, mode: PayMode, bucket: Bucket) -> Resolution {
        let (weights, source) = self
            .overridden(entry)
            .or_else(|| self.income_ratio(mode))
            .or_else(|| self.agreed(entry, bucket.category))
            .unwrap_or_else(|| self.equal());
        Resolution { bucket, weights, source }
    }

    fn overridden(&self, entry: &Entry) -> Option<(Vec<Weight>, ShareSource)> {
        let shares = self.snap.overrides.get(&entry.id)?;
        Some((Self::to_weights(shares), ShareSource::Override))
    }

    fn income_ratio(&self, mode: PayMode) -> Option<(Vec<Weight>, ShareSource)> {
        let usable = mode == PayMode::IncomeRatio && self.income_weights.iter().any(|w| w.weight > 0.0);
        usable.then(|| (self.income_weights.clone(), ShareSource::IncomeRatio))
    }

    fn agreed(&self, entry: &Entry, category: CategoryId) -> Option<(Vec<Weight>, ShareSource)> {
        let agreement = self.snap.agreement_for(category, entry.date)?;
        Some((Self::to_weights(&agreement.shares), ShareSource::Agreement))
    }

    fn equal(&self) -> (Vec<Weight>, ShareSource) {
        let weights = self.participants.iter().map(|m| Weight { member: *m, weight: 1.0 }).collect();
        (weights, ShareSource::Equal)
    }

    fn to_weights(shares: &[Share]) -> Vec<Weight> {
        shares.iter().map(|s| Weight { member: s.member, weight: s.pct }).collect()
    }
}
