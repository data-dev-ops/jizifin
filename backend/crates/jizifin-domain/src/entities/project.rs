use super::parse_variant;
use crate::ids::{CategoryId, MemberId, ProjectId, TagId};
use crate::{Cents, DomainError, Month};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Origin of a stored budget limit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetSource {
    /// Entered by a member.
    Manual,
    /// Accepted from an Auditor proposal.
    AcceptedProposal,
}

impl BudgetSource {
    const TABLE: [(&'static str, Self); 2] =
        [("manual", Self::Manual), ("accepted_proposal", Self::AcceptedProposal)];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("budget source", raw, &Self::TABLE)
    }
}

/// Monthly spending limit for a category.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    /// Category limited.
    pub category: CategoryId,
    /// Month the limit applies to.
    pub month: Month,
    /// Limit.
    pub limit: Cents,
    /// Origin.
    pub source: BudgetSource,
}

/// A savings or spending target shared by members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Surrogate key.
    pub id: ProjectId,
    /// Target amount.
    pub target: Cents,
    /// Participating members.
    pub members: Vec<MemberId>,
}

/// Event tag with an optional active window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// Surrogate key.
    pub id: TagId,
    /// First allowed date.
    pub valid_from: Option<NaiveDate>,
    /// Last allowed date.
    pub valid_to: Option<NaiveDate>,
}

impl Tag {
    /// True when an entry dated `date` may carry this tag.
    pub fn allows(&self, date: NaiveDate) -> bool {
        self.valid_from.is_none_or(|from| from <= date) && self.valid_to.is_none_or(|to| date <= to)
    }
}
