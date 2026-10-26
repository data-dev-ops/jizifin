//! The Auditor façade held in application state.

use crate::budget::{self, BudgetEvaluator, BudgetProposal, BudgetProposer, BudgetReport};
use crate::cache::{DataVersion, SnapshotCache};
use crate::cashflow::{self, CashFlow, CashFlowCalculator};
use crate::deposit::{self, DepositProposal, DepositProposer};
use crate::income::{IncomeResolver, MemberIncome};
use crate::invariant::InvariantSet;
use crate::joint::{self, JointBalanceCalculator, JointDashboard};
use crate::project::{self, ProjectSettlement, ProjectSettlementCalculator};
use crate::ratios::{self, SalaryRatioCalculator, SalaryRatios};
use crate::settlement::{self, Settlement, SettlementCalculator};
use crate::splits::{self, SplitOverview, SplitOverviewCalculator, SplitProposal, SplitProposer, SplitStrategy};
use crate::{AuditError, AuditScope, AuditedResult, Snapshot, SnapshotLoader};
use jizifin_domain::entities::MonthSeal;
use jizifin_domain::Cents;
use crate::shares::EntryShares;
use jizifin_domain::ids::{AccountId, EntryId, MemberId, ProjectId};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

/// A sealed month plus whether today's recomputation still matches it.
#[derive(Clone, Debug, Serialize)]
pub struct SealedSettlement {
    /// The stored seal (the authoritative result).
    pub seal: MonthSeal,
    /// True when recomputing from current data yields a different settlement.
    pub drift: bool,
}

/// Snapshots kept in memory.
const CACHE_CAPACITY: usize = 64;

/// Sole owner of contentious calculations. Every public method returns a verified value.
pub struct Auditor {
    loader: Arc<dyn SnapshotLoader>,
    version: Arc<DataVersion>,
    cache: SnapshotCache,
}

impl Auditor {
    /// Auditor reading through `loader`, invalidated by `version`.
    pub fn new(loader: Arc<dyn SnapshotLoader>, version: Arc<DataVersion>) -> Self {
        Self { loader, version, cache: SnapshotCache::new(CACHE_CAPACITY) }
    }

    /// Peer settlement for a month.
    pub fn settlement(&self, scope: AuditScope) -> Result<AuditedResult<Settlement>, AuditError> {
        self.audit(&scope, &settlement::settlement_invariants(&scope), |snap| {
            Ok(SettlementCalculator::new(snap, &scope).calculate())
        })
    }

    /// Settlement plus one trail line per allocated share.
    pub fn settlement_explained(&self, scope: AuditScope) -> Result<AuditedResult<Settlement>, AuditError> {
        let mut audited = self.settlement(scope.clone())?;
        let (snap, _) = self.snapshot(&scope)?;
        audited.trail = SettlementCalculator::new(&snap, &scope).explain();
        Ok(audited)
    }

    /// Who bears what for one expense. Use a [`AuditScope::month`] containing the entry's date.
    pub fn entry_shares(&self, scope: AuditScope, id: EntryId) -> Result<EntryShares, AuditError> {
        let (snap, _) = self.snapshot(&scope)?;
        let shares = SettlementCalculator::new(&snap, &scope).entry_shares(id).ok_or(AuditError::UnknownEntry(id))?;
        match shares.shares_total() == shares.total {
            true => Ok(shares),
            false => Err(AuditError::InvariantFailed("entry_shares_sum_to_total")),
        }
    }

    /// Re-verifies a seal against current data. The seal stays authoritative and covers the whole
    /// household, so a member filter on `scope` is dropped before comparing.
    pub fn sealed_settlement(&self, mut scope: AuditScope, seal: MonthSeal) -> Result<SealedSettlement, AuditError> {
        scope.members = None;
        let live = serde_json::to_value(&self.settlement(scope)?.value).map_err(|e| AuditError::Load(e.to_string()))?;
        let stored: serde_json::Value = serde_json::from_str(&seal.result_json).map_err(|e| AuditError::Load(e.to_string()))?;
        Ok(SealedSettlement { drift: live != stored, seal })
    }

    /// Proposed deposits for a joint account, optionally also rounded up to `round_to`.
    pub fn deposit_proposal(&self, scope: AuditScope, account: AccountId, round_to: Option<Cents>) -> Result<AuditedResult<DepositProposal>, AuditError> {
        self.audit(&scope, &deposit::deposit_invariants(), |snap| {
            Ok(DepositProposer::new(snap, snap.account(account)?, scope.month).rounded_to(round_to).propose())
        })
    }

    /// Joint-account dashboard. Use [`AuditScope::account_to_date`].
    pub fn joint_dashboard(&self, scope: AuditScope, account: AccountId) -> Result<AuditedResult<JointDashboard>, AuditError> {
        self.audit(&scope, &joint::joint_invariants(), |snap| {
            Ok(JointBalanceCalculator::new(snap, &scope, snap.account(account)?).calculate())
        })
    }

    /// Budget usage for a month.
    pub fn budget_status(&self, scope: AuditScope) -> Result<AuditedResult<BudgetReport>, AuditError> {
        self.audit(&scope, &budget::status_invariants(), |snap| Ok(BudgetEvaluator::new(snap, &scope).evaluate()))
    }

    /// Budget proposals. Use [`AuditScope::trailing`] with [`budget::PROPOSAL_LOOKBACK_MONTHS`].
    pub fn budget_proposals(&self, scope: AuditScope) -> Result<AuditedResult<Vec<BudgetProposal>>, AuditError> {
        self.audit(&scope, &budget::proposal_invariants(), |snap| Ok(BudgetProposer::new(snap, &scope).propose()))
    }

    /// Whole-percent split proposal from income.
    pub fn salary_ratios(&self, scope: AuditScope) -> Result<AuditedResult<SalaryRatios>, AuditError> {
        self.audit(&scope, &ratios::ratio_invariants(), |snap| Ok(SalaryRatioCalculator::new(snap, &scope).calculate()))
    }

    /// Project balance sheet. Use [`AuditScope::project`].
    pub fn project_settlement(&self, scope: AuditScope, id: ProjectId) -> Result<AuditedResult<ProjectSettlement>, AuditError> {
        self.audit(&scope, &project::project_invariants(), |snap| {
            Ok(ProjectSettlementCalculator::new(snap, snap.project(id)?).calculate())
        })
    }

    /// Effective income per member.
    pub fn income(&self, scope: AuditScope) -> Result<AuditedResult<BTreeMap<MemberId, MemberIncome>>, AuditError> {
        self.audit(&scope, &InvariantSet::new(Vec::new()), |snap| Ok(IncomeResolver::new(snap, scope.month).resolve()))
    }

    /// Effective split of every expense category in the scope's month.
    pub fn split_overview(&self, scope: AuditScope) -> Result<AuditedResult<SplitOverview>, AuditError> {
        self.audit(&scope, &splits::overview_invariants(), |snap| Ok(SplitOverviewCalculator::new(snap, &scope).calculate()))
    }

    /// Whole-percent split proposal over the scoped members.
    pub fn split_proposal(&self, scope: AuditScope, strategy: SplitStrategy) -> Result<AuditedResult<SplitProposal>, AuditError> {
        self.audit(&scope, &splits::proposal_invariants(), |snap| Ok(SplitProposer::new(snap, &scope).propose(strategy)))
    }

    /// Income against spending for the scoped members.
    pub fn cash_flow(&self, scope: AuditScope) -> Result<AuditedResult<CashFlow>, AuditError> {
        self.audit(&scope, &cashflow::cash_flow_invariants(), |snap| Ok(CashFlowCalculator::new(snap, &scope).calculate()))
    }

    fn audit<T>(
        &self,
        scope: &AuditScope,
        invariants: &InvariantSet<T>,
        calculate: impl FnOnce(&Snapshot) -> Result<T, AuditError>,
    ) -> Result<AuditedResult<T>, AuditError> {
        let (snap, data_version) = self.snapshot(scope)?;
        let value = calculate(&snap)?;
        let checks = invariants.verify(&value, &snap)?;
        Ok(AuditedResult { value, inputs_hash: snap.inputs_hash(), data_version, checks, trail: Vec::new() })
    }

    fn snapshot(&self, scope: &AuditScope) -> Result<(Arc<Snapshot>, u64), AuditError> {
        let version = self.version.current();
        if let Some(snap) = self.cache.get(scope, version) {
            return Ok((snap, version));
        }
        let snap = Arc::new(self.loader.load(scope)?);
        self.cache.put(scope.clone(), version, Arc::clone(&snap));
        Ok((snap, version))
    }
}
