use super::parse_variant;
use crate::ids::{AccountId, CategoryId, EntryId, MemberId, ProjectId, TagId};
use crate::{Cents, DomainError};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Ledger entry type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Money spent.
    Expense,
    /// Money earned.
    Income,
    /// Member deposit into a joint account.
    Deposit,
    /// Signed joint-account balance correction.
    Correction,
}

impl EntryKind {
    const TABLE: [(&'static str, Self); 4] = [
        ("expense", Self::Expense),
        ("income", Self::Income),
        ("deposit", Self::Deposit),
        ("correction", Self::Correction),
    ];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("entry kind", raw, &Self::TABLE)
    }
}

/// One row of the unified ledger. Text fields stay in the store; the Auditor needs numbers and keys only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Surrogate key, also the allocation tie-break seed.
    pub id: EntryId,
    /// Entry type.
    pub kind: EntryKind,
    /// Booking date.
    pub date: NaiveDate,
    /// Signed amount; refunds are negative expenses.
    pub amount: Cents,
    /// Payer (expense), earner (income) or depositor (deposit).
    pub member: Option<MemberId>,
    /// Category for expenses and income.
    pub category: Option<CategoryId>,
    /// Joint account charged or credited.
    pub account: Option<AccountId>,
    /// Linked project.
    pub project: Option<ProjectId>,
    /// Linked tag.
    pub tag: Option<TagId>,
}

impl Entry {
    /// True for an expense charged to a joint account.
    pub fn is_joint_expense(&self) -> bool {
        self.kind == EntryKind::Expense && self.account.is_some()
    }
}

/// Per-entry custom split replacing every other split source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShareOverride {
    /// Entry being overridden.
    pub entry: EntryId,
    /// Member share.
    pub share: super::Share,
}
