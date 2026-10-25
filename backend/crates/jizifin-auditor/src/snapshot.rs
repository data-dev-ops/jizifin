//! Immutable in-memory copy of the rows a calculation needs.

use crate::{AuditError, AuditScope};
use chrono::NaiveDate;
use jizifin_domain::entities::{
    Account, Agreement, Budget, Category, Entry, EntryKind, IncomeOverride, Job, Member, OffsetRule, Project, Share,
};
use jizifin_domain::ids::{AccountId, CategoryId, EntryId, MemberId, ProjectId};
use jizifin_domain::Month;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Port implemented by the store. Loads raw rows only; it never computes contentious values.
pub trait SnapshotLoader: Send + Sync {
    /// Loads every row relevant to `scope`.
    fn load(&self, scope: &AuditScope) -> Result<Snapshot, AuditError>;
}

/// Raw inputs for one audit.
#[derive(Clone, Debug, Default, Serialize)]
pub struct Snapshot {
    /// All members.
    pub members: Vec<Member>,
    /// All categories.
    pub categories: BTreeMap<CategoryId, Category>,
    /// All split agreements.
    pub agreements: Vec<Agreement>,
    /// Ledger entries inside the scope period.
    pub entries: Vec<Entry>,
    /// Per-entry share overrides.
    pub overrides: BTreeMap<EntryId, Vec<Share>>,
    /// All jobs.
    pub jobs: Vec<Job>,
    /// Income overrides inside the scope period.
    pub income_overrides: Vec<IncomeOverride>,
    /// All joint accounts.
    pub accounts: Vec<Account>,
    /// Active offset rules.
    pub offset_rules: Vec<OffsetRule>,
    /// Budgets inside the scope period.
    pub budgets: Vec<Budget>,
    /// All projects.
    pub projects: Vec<Project>,
}

impl Snapshot {
    /// Hex SHA-256 over the canonical JSON form of the snapshot.
    pub fn inputs_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        hex::encode(Sha256::digest(&bytes))
    }

    /// Category by id.
    pub fn category(&self, id: CategoryId) -> Option<&Category> {
        self.categories.get(&id)
    }

    /// Joint account by id.
    pub fn account(&self, id: AccountId) -> Result<&Account, AuditError> {
        self.accounts.iter().find(|a| a.id == id).ok_or(AuditError::UnknownAccount(id))
    }

    /// Highest-precedence agreement covering `date` for `category`.
    pub fn agreement_for(&self, category: CategoryId, date: NaiveDate) -> Option<&Agreement> {
        self.agreements
            .iter()
            .filter(|a| a.category == category && a.covers(date))
            .max_by_key(|a| a.precedence())
    }

    /// Project by id.
    pub fn project(&self, id: ProjectId) -> Result<&Project, AuditError> {
        self.projects.iter().find(|p| p.id == id).ok_or(AuditError::UnknownProject(id))
    }

    /// True when any joint account charges `category`.
    pub fn is_joint_category(&self, category: CategoryId) -> bool {
        self.accounts.iter().any(|a| a.charges(category))
    }

    /// Ids of active members.
    pub fn active_members(&self) -> Vec<MemberId> {
        self.members.iter().filter(|m| m.is_active).map(|m| m.id).collect()
    }

    /// Entries of `kind` dated inside `month`.
    pub fn entries_in(&self, kind: EntryKind, month: Month) -> impl Iterator<Item = &Entry> {
        let period = month.period();
        self.entries.iter().filter(move |e| e.kind == kind && period.contains(e.date))
    }
}
