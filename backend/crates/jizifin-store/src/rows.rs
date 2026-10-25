//! Row → domain mappers and the column lists they expect. Enums and ciphertext are validated here.

use duckdb::Row;
use duckdb::types::Type;
use jizifin_domain::entities::{
    Account, AccountCategory, AccountMember, Agreement, Budget, BudgetSource, Category, CategoryKind, DepositMode,
    Entry, EntryKind, Frequency, IncomeOverride, Job, Member, MonthSeal, OffsetRule, PayMode, Project, Recurrence,
    RecurringTemplate, Share, Tag,
};
use jizifin_domain::ids::{
    AccountId, AgreementId, CategoryId, EntryId, JobId, MemberId, OffsetRuleId, ProjectId, RecurringId, TagId,
};
use jizifin_domain::{Cents, Ciphertext, DomainError, Month};

type R<T> = duckdb::Result<T>;

pub const MEMBER_COLS: &str = "id, name_ct, color, is_active";
pub const CATEGORY_COLS: &str = "id, kind, name_ct, pay_mode";
pub const AGREEMENT_COLS: &str = "id, category_id, valid_from, valid_to, note_ct";
pub const ENTRY_COLS: &str =
    "id, kind, entry_date, amount_cents, member_id, category_id, account_id, project_id, tag_id, recurring_id, name_ct, note_ct";
pub const JOB_COLS: &str = "id, member_id, amount_cents, frequency, valid_from, valid_to, name_ct, notes_ct";
pub const INCOME_OVERRIDE_COLS: &str = "member_id, month, amount_cents, note_ct";
pub const ACCOUNT_COLS: &str = "id, name_ct, safety_margin_pct, deposit_mode";
pub const ACCOUNT_MEMBER_COLS: &str = "account_id, member_id, fixed_deposit_cents, due_day";
pub const ACCOUNT_CATEGORY_COLS: &str = "account_id, category_id, expected_cents";
pub const OFFSET_RULE_COLS: &str =
    "id, is_active, credit_member_id, credit_category_id, debit_member_id, debit_category_id";
pub const BUDGET_COLS: &str = "category_id, month, limit_cents, source";
pub const PROJECT_COLS: &str = "id, name_ct, color, target_cents";
pub const TAG_COLS: &str = "id, name_ct, description_ct, valid_from, valid_to";
pub const RECURRING_COLS: &str = "id, kind, amount_cents, member_id, category_id, account_id, day_of_month, \
    frequency, valid_from, valid_to, name_ct, is_active";
pub const SEAL_COLS: &str = "month, CAST(result_json AS VARCHAR), inputs_hash, sealed_at";

fn domain<T>(idx: usize, parsed: Result<T, DomainError>) -> R<T> {
    parsed.map_err(|e| duckdb::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e)))
}

fn text<T>(row: &Row<'_>, idx: usize, parse: fn(&str) -> Result<T, DomainError>) -> R<T> {
    let raw: String = row.get(idx)?;
    domain(idx, parse(&raw))
}

fn ct(row: &Row<'_>, idx: usize) -> R<Ciphertext> {
    domain(idx, Ciphertext::parse(row.get::<_, String>(idx)?))
}

fn opt_ct(row: &Row<'_>, idx: usize) -> R<Option<Ciphertext>> {
    row.get::<_, Option<String>>(idx)?.map(|raw| domain(idx, Ciphertext::parse(raw))).transpose()
}

fn cents(row: &Row<'_>, idx: usize) -> R<Cents> {
    Ok(Cents::new(row.get(idx)?))
}

fn opt_id<T>(row: &Row<'_>, idx: usize, wrap: fn(i64) -> T) -> R<Option<T>> {
    Ok(row.get::<_, Option<i64>>(idx)?.map(wrap))
}

/// [`MEMBER_COLS`]
pub fn member(row: &Row<'_>) -> R<Member> {
    Ok(Member { id: MemberId(row.get(0)?), name: ct(row, 1)?, color: row.get(2)?, is_active: row.get(3)? })
}

/// [`CATEGORY_COLS`]
pub fn category(row: &Row<'_>) -> R<Category> {
    let (kind, pay_mode) = (text(row, 1, CategoryKind::parse)?, text(row, 3, PayMode::parse)?);
    Ok(Category { id: CategoryId(row.get(0)?), kind, name: ct(row, 2)?, pay_mode })
}

/// [`AGREEMENT_COLS`] (shares attached later)
pub fn agreement(row: &Row<'_>) -> R<Agreement> {
    Ok(Agreement {
        id: AgreementId(row.get(0)?),
        category: CategoryId(row.get(1)?),
        valid_from: row.get(2)?,
        valid_to: row.get(3)?,
        note: opt_ct(row, 4)?,
        shares: Vec::new(),
    })
}

/// `owner_id, member_id, pct`
pub fn owned_share(row: &Row<'_>) -> R<(i64, Share)> {
    Ok((row.get(0)?, Share { member: MemberId(row.get(1)?), pct: row.get(2)? }))
}

/// [`ENTRY_COLS`]
pub fn entry(row: &Row<'_>) -> R<Entry> {
    Ok(Entry {
        id: EntryId(row.get(0)?),
        kind: text(row, 1, EntryKind::parse)?,
        date: row.get(2)?,
        amount: cents(row, 3)?,
        member: opt_id(row, 4, MemberId)?,
        category: opt_id(row, 5, CategoryId)?,
        account: opt_id(row, 6, AccountId)?,
        project: opt_id(row, 7, ProjectId)?,
        tag: opt_id(row, 8, TagId)?,
        recurring: opt_id(row, 9, RecurringId)?,
        name: opt_ct(row, 10)?,
        note: opt_ct(row, 11)?,
    })
}

/// [`JOB_COLS`]
pub fn job(row: &Row<'_>) -> R<Job> {
    Ok(Job {
        id: JobId(row.get(0)?),
        member: MemberId(row.get(1)?),
        amount: cents(row, 2)?,
        frequency: text(row, 3, Frequency::parse)?,
        valid_from: row.get(4)?,
        valid_to: row.get(5)?,
        name: ct(row, 6)?,
        notes: opt_ct(row, 7)?,
    })
}

/// [`INCOME_OVERRIDE_COLS`]
pub fn income_override(row: &Row<'_>) -> R<IncomeOverride> {
    Ok(IncomeOverride {
        member: MemberId(row.get(0)?),
        month: Month::of(row.get(1)?),
        amount: cents(row, 2)?,
        note: opt_ct(row, 3)?,
    })
}

/// [`ACCOUNT_COLS`] (members and categories attached later)
pub fn account(row: &Row<'_>) -> R<Account> {
    Ok(Account {
        id: AccountId(row.get(0)?),
        name: ct(row, 1)?,
        safety_margin_pct: row.get(2)?,
        deposit_mode: text(row, 3, DepositMode::parse)?,
        members: Vec::new(),
        categories: Vec::new(),
    })
}

/// [`ACCOUNT_MEMBER_COLS`]
pub fn account_member(row: &Row<'_>) -> R<(AccountId, AccountMember)> {
    let fixed_deposit = row.get::<_, Option<i64>>(2)?.map(Cents::new);
    let member = AccountMember { member: MemberId(row.get(1)?), fixed_deposit, due_day: row.get(3)? };
    Ok((AccountId(row.get(0)?), member))
}

/// [`ACCOUNT_CATEGORY_COLS`]
pub fn account_category(row: &Row<'_>) -> R<(AccountId, AccountCategory)> {
    let category = AccountCategory { category: CategoryId(row.get(1)?), expected: cents(row, 2)? };
    Ok((AccountId(row.get(0)?), category))
}

/// [`OFFSET_RULE_COLS`]
pub fn offset_rule(row: &Row<'_>) -> R<OffsetRule> {
    Ok(OffsetRule {
        id: OffsetRuleId(row.get(0)?),
        is_active: row.get(1)?,
        credit_member: MemberId(row.get(2)?),
        credit_category: CategoryId(row.get(3)?),
        debit_member: MemberId(row.get(4)?),
        debit_category: CategoryId(row.get(5)?),
    })
}

/// [`BUDGET_COLS`]
pub fn budget(row: &Row<'_>) -> R<Budget> {
    Ok(Budget {
        category: CategoryId(row.get(0)?),
        month: Month::of(row.get(1)?),
        limit: cents(row, 2)?,
        source: text(row, 3, BudgetSource::parse)?,
    })
}

/// [`PROJECT_COLS`] (members attached later)
pub fn project(row: &Row<'_>) -> R<Project> {
    Ok(Project {
        id: ProjectId(row.get(0)?),
        name: ct(row, 1)?,
        color: row.get(2)?,
        target: cents(row, 3)?,
        members: Vec::new(),
    })
}

/// [`TAG_COLS`]
pub fn tag(row: &Row<'_>) -> R<Tag> {
    Ok(Tag {
        id: TagId(row.get(0)?),
        name: ct(row, 1)?,
        description: opt_ct(row, 2)?,
        valid_from: row.get(3)?,
        valid_to: row.get(4)?,
    })
}

/// [`RECURRING_COLS`]
pub fn recurring(row: &Row<'_>) -> R<RecurringTemplate> {
    Ok(RecurringTemplate {
        id: RecurringId(row.get(0)?),
        kind: text(row, 1, EntryKind::parse)?,
        amount: cents(row, 2)?,
        member: MemberId(row.get(3)?),
        category: CategoryId(row.get(4)?),
        account: opt_id(row, 5, AccountId)?,
        day_of_month: row.get(6)?,
        recurrence: text(row, 7, Recurrence::parse)?,
        valid_from: row.get(8)?,
        valid_to: row.get(9)?,
        name: ct(row, 10)?,
        is_active: row.get(11)?,
    })
}

/// [`SEAL_COLS`]
pub fn seal(row: &Row<'_>) -> R<MonthSeal> {
    let sealed_at: chrono::NaiveDateTime = row.get(3)?;
    Ok(MonthSeal {
        month: Month::of(row.get(0)?),
        result_json: row.get(1)?,
        inputs_hash: row.get(2)?,
        sealed_at: sealed_at.and_utc(),
    })
}
