use crate::ids::{CategoryId, MemberId, OffsetRuleId};
use crate::Month;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Netting rule: what `credit_member` paid in `credit_category` offsets what
/// `debit_member` paid in `debit_category`, up to the smaller of the two.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OffsetRule {
    /// Surrogate key.
    pub id: OffsetRuleId,
    /// Inactive rules are kept but not applied.
    pub is_active: bool,
    /// Member whose net position increases.
    pub credit_member: MemberId,
    /// Category paid by the credit member.
    pub credit_category: CategoryId,
    /// Member whose net position decreases.
    pub debit_member: MemberId,
    /// Category paid by the debit member.
    pub debit_category: CategoryId,
}

/// Sealed, immutable settlement of a month.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MonthSeal {
    /// Month sealed.
    pub month: Month,
    /// Serialized settlement at sealing time.
    pub result_json: String,
    /// Hex SHA-256 of the snapshot the settlement was computed from.
    pub inputs_hash: String,
    /// Sealing timestamp.
    pub sealed_at: DateTime<Utc>,
}
