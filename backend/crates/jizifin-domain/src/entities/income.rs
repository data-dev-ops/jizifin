use super::{label_of, parse_variant};
use crate::ids::{JobId, MemberId};
use crate::{Cents, Ciphertext, DomainError, Month};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Pay frequency of a job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    /// Paid monthly.
    Monthly,
    /// Paid every week.
    Weekly,
    /// Paid every two weeks.
    Biweekly,
    /// Paid yearly.
    Annual,
}

impl Frequency {
    const TABLE: [(&'static str, Self); 4] = [
        ("monthly", Self::Monthly),
        ("weekly", Self::Weekly),
        ("biweekly", Self::Biweekly),
        ("annual", Self::Annual),
    ];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("frequency", raw, &Self::TABLE)
    }

    /// Stored label.
    pub fn label(self) -> &'static str {
        label_of(self, &Self::TABLE)
    }

    /// Payments per year.
    pub fn periods_per_year(self) -> i64 {
        match self {
            Self::Monthly => 12,
            Self::Weekly => 52,
            Self::Biweekly => 26,
            Self::Annual => 1,
        }
    }
}

/// An employment contract producing regular income.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Surrogate key.
    pub id: JobId,
    /// Earner.
    pub member: MemberId,
    /// Amount per pay period.
    pub amount: Cents,
    /// Pay period.
    pub frequency: Frequency,
    /// Contract start.
    pub valid_from: NaiveDate,
    /// Contract end; `None` means ongoing.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted employer name.
    pub name: Ciphertext,
    /// Encrypted notes.
    pub notes: Option<Ciphertext>,
}

impl Job {
    /// True when the contract overlaps `month`.
    pub fn active_in(&self, month: Month) -> bool {
        month.period().overlaps(self.valid_from, self.valid_to)
    }
}

/// Month-specific replacement of a member's contract salary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IncomeOverride {
    /// Earner.
    pub member: MemberId,
    /// Month replaced.
    pub month: Month,
    /// Replacement amount.
    pub amount: Cents,
    /// Encrypted reason.
    pub note: Option<Ciphertext>,
}
