//! Resolves who bears which share of an expense.
//! Precedence: personal pay → entry override → income ratio → SCD2 agreement → equal split.

use crate::Snapshot;
use crate::allocation::Weight;
use jizifin_domain::entities::{Entry, PayMode, Share};
use jizifin_domain::ids::{CategoryId, MemberId};
use serde::Serialize;

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
