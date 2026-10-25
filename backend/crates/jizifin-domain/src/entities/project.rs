use super::{label_of, parse_variant};
use crate::ids::{BudgetId, CategoryId, MemberId, ProjectId, TagId};
use crate::{Cents, Ciphertext, DomainError, Month};
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

    /// Stored label.
    pub fn label(self) -> &'static str {
        label_of(self, &Self::TABLE)
    }
}

/// Monthly spending limit for a category over a range of months (SCD2).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Budget {
    /// Surrogate key.
    pub id: BudgetId,
    /// Category limited.
    pub category: CategoryId,
    /// First month the limit applies to.
    pub valid_from: Month,
    /// Last month; `None` means standing.
    pub valid_to: Option<Month>,
    /// Limit per month.
    pub limit: Cents,
    /// Origin.
    pub source: BudgetSource,
}

impl Budget {
    /// True when the limit applies to `month`.
    pub fn covers(&self, month: Month) -> bool {
        self.valid_from <= month && self.valid_to.is_none_or(|end| month <= end)
    }

    /// Precedence key: bounded beats standing, then latest start, then highest id.
    pub fn precedence(&self) -> (bool, Month, BudgetId) {
        (self.valid_to.is_some(), self.valid_from, self.id)
    }
}

/// A savings or spending target shared by members.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Surrogate key.
    pub id: ProjectId,
    /// Encrypted name.
    pub name: Ciphertext,
    /// Hex colour.
    pub color: String,
    /// Target amount.
    pub target: Cents,
    /// Target date.
    pub target_date: Option<NaiveDate>,
    /// Participating members.
    pub members: Vec<MemberId>,
}

/// Event tag with an optional active window.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    /// Surrogate key.
    pub id: TagId,
    /// Encrypted name.
    pub name: Ciphertext,
    /// Encrypted description.
    pub description: Option<Ciphertext>,
    /// Hex colour.
    pub color: String,
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
