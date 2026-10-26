//! Write payloads. Domain types do the format validation during deserialization;
//! `validate` methods enforce the cross-field rules.

use crate::StoreError;
use chrono::NaiveDate;
use jizifin_domain::entities::{
    AccountCategory, AccountMember, BudgetSource, CategoryKind, DepositMode, EntryKind, Frequency, Job, PayMode, Recurrence,
    Share,
};
use jizifin_domain::ids::{AccountId, CategoryId, EntryId, MemberId, ProjectId, TagId};
use jizifin_domain::{Cents, Ciphertext, Month, Period};
use serde::Deserialize;
use std::collections::BTreeSet;

/// Allowed deviation of a share total from 100 %.
const PCT_TOLERANCE: f64 = 1e-4;

fn default_color() -> String {
    "#64748b".into()
}

fn tag_color() -> String {
    "#f59e0b".into()
}

fn manual() -> BudgetSource {
    BudgetSource::Manual
}

fn yes() -> bool {
    true
}

fn shared() -> PayMode {
    PayMode::Shared
}

fn window(from: NaiveDate, to: Option<NaiveDate>) -> Result<(), StoreError> {
    to.map_or(Ok(()), |to| Period::new(from, to).map(|_| ()).map_err(Into::into))
}

/// Shares must name distinct members and total 100 %.
pub(crate) fn validate_shares(shares: &[Share]) -> Result<(), StoreError> {
    let distinct: BTreeSet<MemberId> = shares.iter().map(|s| s.member).collect();
    let total: f64 = shares.iter().map(|s| s.pct).sum();
    match distinct.len() == shares.len() && (total - 100.0).abs() <= PCT_TOLERANCE {
        true => Ok(()),
        false => Err(StoreError::Invalid(format!("shares must name distinct members and total 100 (got {total})"))),
    }
}

/// Member create/replace.
#[derive(Debug, Deserialize)]
pub struct MemberInput {
    /// Encrypted name.
    pub name: Ciphertext,
    /// Hex colour.
    #[serde(default = "default_color")]
    pub color: String,
    /// Active flag.
    #[serde(default = "yes")]
    pub is_active: bool,
}

/// Category create/replace.
#[derive(Debug, Deserialize)]
pub struct CategoryInput {
    /// Expense or income.
    pub kind: CategoryKind,
    /// Encrypted name.
    pub name: Ciphertext,
    /// Liability sharing mode.
    #[serde(default = "shared")]
    pub pay_mode: PayMode,
}

/// Agreement create/replace.
#[derive(Debug, Deserialize)]
pub struct AgreementInput {
    /// First effective day.
    pub valid_from: NaiveDate,
    /// Last effective day.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted note.
    pub note: Option<Ciphertext>,
    /// Member shares, total 100.
    pub shares: Vec<Share>,
}

impl AgreementInput {
    /// Window and share-total rules.
    pub fn validate(&self) -> Result<(), StoreError> {
        window(self.valid_from, self.valid_to)?;
        validate_shares(&self.shares)
    }
}

/// Joint account create/replace, including its roster and categories.
#[derive(Debug, Deserialize)]
pub struct AccountInput {
    /// Encrypted name.
    pub name: Ciphertext,
    /// Safety margin in percent.
    #[serde(default)]
    pub safety_margin_pct: f64,
    /// Gross expected monthly cost replacing the category sum.
    #[serde(default)]
    pub expected_override: Option<Cents>,
    /// Deposit division mode.
    pub deposit_mode: DepositMode,
    /// Participants.
    pub members: Vec<AccountMember>,
    /// Categories charged to the account.
    pub categories: Vec<AccountCategory>,
}

impl AccountInput {
    /// Margin, due-day and fixed-mode rules.
    pub fn validate(&self) -> Result<(), StoreError> {
        let days_ok = self.members.iter().all(|m| (1..=31).contains(&m.due_day));
        let fixed_ok = self.deposit_mode != DepositMode::Fixed || self.members.iter().all(|m| m.fixed_deposit.is_some());
        match self.safety_margin_pct >= 0.0 && days_ok && fixed_ok {
            true => Ok(()),
            false => Err(StoreError::Invalid("invalid margin, due day or missing fixed deposit".into())),
        }
    }
}

/// Ledger entry create/replace.
#[derive(Debug, Deserialize)]
pub struct EntryInput {
    /// Entry type.
    pub kind: EntryKind,
    /// Booking date.
    pub date: NaiveDate,
    /// Signed amount.
    pub amount: Cents,
    /// Payer, earner or depositor.
    pub member: Option<MemberId>,
    /// Category (expense and income).
    pub category: Option<CategoryId>,
    /// Joint account.
    pub account: Option<AccountId>,
    /// Project.
    pub project: Option<ProjectId>,
    /// Tag.
    pub tag: Option<TagId>,
    /// Encrypted description.
    pub name: Option<Ciphertext>,
    /// Encrypted note.
    pub note: Option<Ciphertext>,
    /// Custom split; empty means none.
    #[serde(default)]
    pub overrides: Vec<Share>,
}

impl EntryInput {
    /// Shape rules per entry kind.
    pub fn validate(&self) -> Result<(), StoreError> {
        let ledger = matches!(self.kind, EntryKind::Expense | EntryKind::Income);
        let shaped = match ledger {
            true => self.member.is_some() && self.category.is_some(),
            false => self.account.is_some() && (self.kind == EntryKind::Correction || self.member.is_some()),
        };
        if !shaped {
            return Err(StoreError::Invalid(format!("{:?} entry is missing member, category or account", self.kind)));
        }
        match self.overrides.is_empty() {
            true => Ok(()),
            false => validate_shares(&self.overrides),
        }
    }
}

/// One duplicate probe: same category and amount in the same month.
#[derive(Debug, Deserialize)]
pub struct DuplicateProbe {
    /// Category.
    pub category: CategoryId,
    /// Amount.
    pub amount: Cents,
    /// Date whose month is searched.
    pub date: NaiveDate,
    /// Entry being edited, excluded from matches.
    pub exclude: Option<EntryId>,
}

/// Pay change from `effective`: the current job ends the day before, a copy starts with the new amount.
#[derive(Debug, Deserialize)]
pub struct JobAdjustment {
    /// First day of the new rate.
    pub effective: NaiveDate,
    /// New amount per period.
    pub amount: Cents,
    /// Encrypted note for the new stream.
    pub notes: Option<Ciphertext>,
}

/// Job create/replace.
#[derive(Debug, Deserialize)]
pub struct JobInput {
    /// Earner.
    pub member: MemberId,
    /// Encrypted employer name.
    pub name: Ciphertext,
    /// Amount per period.
    pub amount: Cents,
    /// Pay period.
    pub frequency: Frequency,
    /// Start.
    pub valid_from: NaiveDate,
    /// End.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted notes.
    pub notes: Option<Ciphertext>,
}

impl From<&Job> for JobInput {
    fn from(job: &Job) -> Self {
        Self {
            member: job.member,
            name: job.name.clone(),
            amount: job.amount,
            frequency: job.frequency,
            valid_from: job.valid_from,
            valid_to: job.valid_to,
            notes: job.notes.clone(),
        }
    }
}

impl JobInput {
    /// Window and amount rules.
    pub fn validate(&self) -> Result<(), StoreError> {
        window(self.valid_from, self.valid_to)?;
        match self.amount.is_negative() {
            true => Err(StoreError::Invalid("job amount must not be negative".into())),
            false => Ok(()),
        }
    }
}

/// Project create/replace.
#[derive(Debug, Deserialize)]
pub struct ProjectInput {
    /// Encrypted name.
    pub name: Ciphertext,
    /// Hex colour.
    #[serde(default = "default_color")]
    pub color: String,
    /// Target amount.
    #[serde(default)]
    pub target: Cents,
    /// Target date.
    pub target_date: Option<NaiveDate>,
    /// Participants.
    #[serde(default)]
    pub members: Vec<MemberId>,
}

/// Tag create/replace.
#[derive(Debug, Deserialize)]
pub struct TagInput {
    /// Encrypted name.
    pub name: Ciphertext,
    /// Encrypted description.
    pub description: Option<Ciphertext>,
    /// Hex colour.
    #[serde(default = "tag_color")]
    pub color: String,
    /// First allowed date.
    pub valid_from: Option<NaiveDate>,
    /// Last allowed date.
    pub valid_to: Option<NaiveDate>,
}

impl TagInput {
    /// Window rule.
    pub fn validate(&self) -> Result<(), StoreError> {
        match (self.valid_from, self.valid_to) {
            (Some(from), to) => window(from, to),
            (None, _) => Ok(()),
        }
    }
}

/// Recurring template create/replace.
#[derive(Debug, Deserialize)]
pub struct RecurringInput {
    /// Expense or income.
    pub kind: EntryKind,
    /// Amount per occurrence.
    pub amount: Cents,
    /// Payer or earner.
    pub member: MemberId,
    /// Category.
    pub category: CategoryId,
    /// Joint account.
    pub account: Option<AccountId>,
    /// Due day.
    pub day_of_month: u32,
    /// Interval.
    pub recurrence: Recurrence,
    /// First occurrence bound.
    pub valid_from: NaiveDate,
    /// Last occurrence bound.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted description.
    pub name: Ciphertext,
    /// Active flag.
    #[serde(default = "yes")]
    pub is_active: bool,
}

impl RecurringInput {
    /// Kind, day and window rules.
    pub fn validate(&self) -> Result<(), StoreError> {
        window(self.valid_from, self.valid_to)?;
        let ok = matches!(self.kind, EntryKind::Expense | EntryKind::Income) && (1..=31).contains(&self.day_of_month);
        match ok {
            true => Ok(()),
            false => Err(StoreError::Invalid("recurring templates must be expense or income with day 1–31".into())),
        }
    }
}

/// Budget create/replace. `valid_to = None` makes it a standing budget.
#[derive(Debug, Deserialize)]
pub struct BudgetInput {
    /// Category limited.
    pub category: CategoryId,
    /// First month.
    pub valid_from: Month,
    /// Last month.
    pub valid_to: Option<Month>,
    /// Limit per month.
    pub limit: Cents,
    /// Origin.
    #[serde(default = "manual")]
    pub source: BudgetSource,
}

impl BudgetInput {
    /// Range and amount rules.
    pub fn validate(&self) -> Result<(), StoreError> {
        let ordered = self.valid_to.is_none_or(|end| end >= self.valid_from);
        match ordered && !self.limit.is_negative() {
            true => Ok(()),
            false => Err(StoreError::Invalid("budget range is inverted or limit is negative".into())),
        }
    }
}

/// Offset rule create.
#[derive(Debug, Deserialize)]
pub struct OffsetRuleInput {
    /// Member whose net increases.
    pub credit_member: MemberId,
    /// Category paid by the credit member.
    pub credit_category: CategoryId,
    /// Member whose net decreases.
    pub debit_member: MemberId,
    /// Category paid by the debit member.
    pub debit_category: CategoryId,
    /// Active flag.
    #[serde(default = "yes")]
    pub is_active: bool,
}
