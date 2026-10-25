//! What slice of the ledger a calculation needs.

use chrono::NaiveDate;
use jizifin_domain::ids::{AccountId, MemberId};
use jizifin_domain::{Month, Period};
use std::collections::BTreeSet;

/// Earliest date the ledger can hold; lower bound for whole-history scopes.
pub const LEDGER_START: NaiveDate = NaiveDate::from_ymd_opt(1, 1, 1).expect("valid date");

/// Inputs that determine which rows the snapshot loads. Also the cache key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AuditScope {
    /// Month being evaluated.
    pub month: Month,
    /// Entry dates loaded.
    pub period: Period,
    /// Optional member filter.
    pub members: Option<BTreeSet<MemberId>>,
    /// Optional joint account.
    pub account: Option<AccountId>,
    /// Evaluation date, used for due-day status.
    pub as_of: NaiveDate,
}

impl AuditScope {
    /// Scope covering exactly one month.
    pub fn month(month: Month, as_of: NaiveDate) -> Self {
        Self { month, period: month.period(), members: None, account: None, as_of }
    }

    /// Scope covering `months` full months before `month`, plus `month` itself.
    pub fn trailing(month: Month, months: u32, as_of: NaiveDate) -> Self {
        let start = (0..months).fold(month, |m, _| m.previous());
        let period = Period { from: start.first_day(), to: month.last_day() };
        Self { period, ..Self::month(month, as_of) }
    }

    /// Scope covering the whole ledger up to the end of `month` for one account.
    pub fn account_to_date(month: Month, account: AccountId, as_of: NaiveDate) -> Self {
        let period = Period { from: LEDGER_START, to: month.last_day() };
        Self { period, account: Some(account), ..Self::month(month, as_of) }
    }

    /// Restricts the scope to a member set; an empty set means no filter.
    pub fn with_members(mut self, members: BTreeSet<MemberId>) -> Self {
        self.members = (!members.is_empty()).then_some(members);
        self
    }
}
