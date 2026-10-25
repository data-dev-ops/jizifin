use super::parse_variant;
use crate::ids::{AccountId, CategoryId, MemberId};
use crate::{Cents, DomainError};
use serde::{Deserialize, Serialize};

/// How proposed deposits are divided between account members.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepositMode {
    /// Equal shares.
    Equal,
    /// Weighted by effective income.
    SalaryRatio,
    /// Each member's configured fixed amount.
    Fixed,
}

impl DepositMode {
    const TABLE: [(&'static str, Self); 3] =
        [("equal", Self::Equal), ("salary_ratio", Self::SalaryRatio), ("fixed", Self::Fixed)];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("deposit mode", raw, &Self::TABLE)
    }
}

/// A member's participation in a joint account.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountMember {
    /// Participant.
    pub member: MemberId,
    /// Amount used when the account is in `Fixed` mode.
    pub fixed_deposit: Option<Cents>,
    /// Day of month the deposit is due.
    pub due_day: u32,
}

/// A category paid from the joint account, with its expected monthly cost.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountCategory {
    /// Category.
    pub category: CategoryId,
    /// Expected monthly cost.
    pub expected: Cents,
}

/// A joint account.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// Surrogate key.
    pub id: AccountId,
    /// Safety margin applied on top of expected costs.
    pub safety_margin_pct: f64,
    /// Deposit division mode.
    pub deposit_mode: DepositMode,
    /// Participants.
    pub members: Vec<AccountMember>,
    /// Categories charged to this account.
    pub categories: Vec<AccountCategory>,
}

impl Account {
    /// Sum of expected category costs.
    pub fn expected_total(&self) -> Cents {
        self.categories.iter().map(|c| c.expected).sum()
    }

    /// True when `category` is charged to this account.
    pub fn charges(&self, category: CategoryId) -> bool {
        self.categories.iter().any(|c| c.category == category)
    }
}
