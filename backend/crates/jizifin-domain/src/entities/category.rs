use super::{label_of, parse_variant};
use crate::ids::{AgreementId, CategoryId, MemberId};
use crate::{Ciphertext, DomainError};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Ledger side a category belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    /// Spending category.
    Expense,
    /// Income category.
    Income,
}

impl CategoryKind {
    const TABLE: [(&'static str, Self); 2] = [("expense", Self::Expense), ("income", Self::Income)];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("category kind", raw, &Self::TABLE)
    }

    /// Stored label.
    pub fn label(self) -> &'static str {
        label_of(self, &Self::TABLE)
    }
}

/// How the liability of an expense in this category is shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayMode {
    /// Agreement shares, else equal shares.
    Shared,
    /// Payer bears 100 %.
    Personal,
    /// Weighted by each member's effective income that month.
    IncomeRatio,
}

impl PayMode {
    const TABLE: [(&'static str, Self); 3] =
        [("shared", Self::Shared), ("personal", Self::Personal), ("income_ratio", Self::IncomeRatio)];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("pay mode", raw, &Self::TABLE)
    }

    /// Stored label.
    pub fn label(self) -> &'static str {
        label_of(self, &Self::TABLE)
    }
}

/// An expense or income category.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Category {
    /// Surrogate key.
    pub id: CategoryId,
    /// Expense or income.
    pub kind: CategoryKind,
    /// Encrypted name.
    pub name: Ciphertext,
    /// Liability sharing mode.
    pub pay_mode: PayMode,
}

/// One member's percentage inside an agreement or override.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Share {
    /// Member bearing the share.
    pub member: MemberId,
    /// Percentage, basis-point precision, never truncated.
    pub pct: f64,
}

/// SCD2 split agreement for a category. Open-ended agreements form the baseline.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Agreement {
    /// Surrogate key.
    pub id: AgreementId,
    /// Category the agreement applies to.
    pub category: CategoryId,
    /// First day the agreement is effective.
    pub valid_from: NaiveDate,
    /// Last effective day; `None` means open-ended.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted note.
    pub note: Option<Ciphertext>,
    /// Member shares.
    pub shares: Vec<Share>,
}

impl Agreement {
    /// True when the agreement is effective on `date`.
    pub fn covers(&self, date: NaiveDate) -> bool {
        self.valid_from <= date && self.valid_to.is_none_or(|end| date <= end)
    }

    /// True for a date-bounded override agreement.
    pub fn is_bounded(&self) -> bool {
        self.valid_to.is_some()
    }

    /// Precedence key: bounded beats open-ended, then latest start, then highest id.
    pub fn precedence(&self) -> (bool, NaiveDate, AgreementId) {
        (self.is_bounded(), self.valid_from, self.id)
    }
}
