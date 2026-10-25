//! Project balance sheet: effective funding (direct or joint-account equity) vs assigned liability.

use crate::allocation::{LargestRemainderAllocator, TieSeed, Weight};
use crate::income::IncomeResolver;
use crate::invariant::{Invariant, InvariantSet};
use crate::settlement::{Debt, DebtSimplifier, debts_settle};
use crate::shares::ShareResolver;
use crate::Snapshot;
use jizifin_domain::entities::{Entry, EntryKind, Project};
use jizifin_domain::ids::{AccountId, MemberId, ProjectId};
use jizifin_domain::{Cents, Month};
use serde::Serialize;
use std::collections::BTreeMap;

type MemberCents = BTreeMap<MemberId, Cents>;

/// One participant's position.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Participant {
    /// Member.
    pub member: MemberId,
    /// Money the member effectively put in.
    pub funding: Cents,
    /// Money the member is liable for.
    pub liability: Cents,
    /// `funding − liability`.
    pub net: Cents,
}

/// Full project settlement.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProjectSettlement {
    /// Project.
    pub project: ProjectId,
    /// Target amount.
    pub target: Cents,
    /// Σ project expenses.
    pub total_spent: Cents,
    /// Positions per member.
    pub participants: Vec<Participant>,
    /// Simplified transfers.
    pub debts: Vec<Debt>,
}

/// Computes [`ProjectSettlement`].
pub struct ProjectSettlementCalculator<'a> {
    snap: &'a Snapshot,
    project: &'a Project,
}

impl<'a> ProjectSettlementCalculator<'a> {
    /// Calculator for `project`.
    pub fn new(snap: &'a Snapshot, project: &'a Project) -> Self {
        Self { snap, project }
    }

    /// Builds the balance sheet.
    pub fn calculate(&self) -> ProjectSettlement {
        let (mut funding, mut liability) = (MemberCents::new(), MemberCents::new());
        for entry in self.expenses() {
            Self::add(&mut funding, self.funding_of(entry));
            Self::add(&mut liability, self.liability_of(entry));
        }
        let participants = self.participants(&funding, &liability);
        let net: MemberCents = participants.iter().map(|p| (p.member, p.net)).collect();
        let members: Vec<MemberId> = net.keys().copied().collect();
        let total_spent = self.expenses().map(|e| e.amount).sum();
        let debts = DebtSimplifier.simplify(&members, &net);
        ProjectSettlement { project: self.project.id, target: self.project.target, total_spent, participants, debts }
    }

    fn expenses(&self) -> impl Iterator<Item = &'a Entry> {
        let id = self.project.id;
        self.snap.entries.iter().filter(move |e| e.kind == EntryKind::Expense && e.project == Some(id))
    }

    fn members(&self) -> Vec<MemberId> {
        match self.project.members.is_empty() {
            false => self.project.members.clone(),
            true => self.expenses().filter_map(|e| e.member).collect::<std::collections::BTreeSet<_>>().into_iter().collect(),
        }
    }

    fn funding_of(&self, entry: &Entry) -> MemberCents {
        match (entry.account, entry.member) {
            (Some(account), _) => self.allocate(entry, &self.equity(account, Month::of(entry.date))),
            (None, Some(payer)) => MemberCents::from([(payer, entry.amount)]),
            (None, None) => MemberCents::new(),
        }
    }

    fn equity(&self, account: AccountId, month: Month) -> Vec<Weight> {
        let period = month.period();
        let deposits = self.snap.entries.iter().filter(|e| e.kind == EntryKind::Deposit && e.account == Some(account) && period.contains(e.date));
        let mut weights: BTreeMap<MemberId, f64> = BTreeMap::new();
        deposits.filter_map(|e| e.member.map(|m| (m, e.amount))).for_each(|(m, c)| *weights.entry(m).or_default() += c.value() as f64);
        match weights.values().any(|w| *w > 0.0) {
            true => weights.into_iter().map(|(member, weight)| Weight { member, weight }).collect(),
            false => self.equal_account_weights(account),
        }
    }

    fn equal_account_weights(&self, account: AccountId) -> Vec<Weight> {
        let members = self.snap.account(account).map(|a| a.members.clone()).unwrap_or_default();
        members.into_iter().map(|m| Weight { member: m.member, weight: 1.0 }).collect()
    }

    fn liability_of(&self, entry: &Entry) -> MemberCents {
        let (Some(payer), Some(category)) = (entry.member, entry.category) else { return MemberCents::new() };
        let income = IncomeResolver::new(self.snap, Month::of(entry.date)).weights();
        let resolution = ShareResolver::new(self.snap, income, self.members()).resolve(entry, payer, category);
        self.allocate(entry, &resolution.weights)
    }

    fn allocate(&self, entry: &Entry, weights: &[Weight]) -> MemberCents {
        let alloc = LargestRemainderAllocator.allocate(entry.amount.value(), weights, &TieSeed::entry(entry.id));
        alloc.iter().map(|(m, u)| (m, Cents::new(u))).collect()
    }

    fn add(into: &mut MemberCents, from: MemberCents) {
        from.into_iter().for_each(|(m, c)| *into.entry(m).or_default() += c);
    }

    fn participants(&self, funding: &MemberCents, liability: &MemberCents) -> Vec<Participant> {
        let mut members = self.members();
        members.extend(funding.keys().chain(liability.keys()).filter(|m| !self.project.members.contains(m)));
        members.sort();
        members.dedup();
        members.into_iter().map(|m| Self::participant(m, funding, liability)).collect()
    }

    fn participant(member: MemberId, funding: &MemberCents, liability: &MemberCents) -> Participant {
        let f = funding.get(&member).copied().unwrap_or_default();
        let l = liability.get(&member).copied().unwrap_or_default();
        Participant { member, funding: f, liability: l, net: f - l }
    }
}

/// Invariant set applied to [`ProjectSettlement`].
pub fn project_invariants() -> InvariantSet<ProjectSettlement> {
    InvariantSet::new(vec![Box::new(FundingAndLiabilityBalance), Box::new(ProjectDebtsSettle)])
}

/// Σ funding = Σ liability = total spent.
struct FundingAndLiabilityBalance;

impl Invariant<ProjectSettlement> for FundingAndLiabilityBalance {
    fn id(&self) -> &'static str {
        "project_funding_liability_balance"
    }
    fn holds(&self, p: &ProjectSettlement, _: &Snapshot) -> bool {
        let funding: Cents = p.participants.iter().map(|x| x.funding).sum();
        let liability: Cents = p.participants.iter().map(|x| x.liability).sum();
        funding == p.total_spent && liability == p.total_spent
    }
}

/// Debts zero every participant's net.
struct ProjectDebtsSettle;

impl Invariant<ProjectSettlement> for ProjectDebtsSettle {
    fn id(&self) -> &'static str {
        "project_debts_settle"
    }
    fn holds(&self, p: &ProjectSettlement, _: &Snapshot) -> bool {
        let net: MemberCents = p.participants.iter().map(|x| (x.member, x.net)).collect();
        debts_settle(&net, &p.debts)
    }
}
