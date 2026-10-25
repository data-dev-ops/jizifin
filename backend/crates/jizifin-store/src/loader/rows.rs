//! Row → domain mappers. Text enums and ciphertext are validated here.

use duckdb::Row;
use duckdb::types::Type;
use jizifin_domain::entities::{
    Account, AccountCategory, AccountMember, Budget, BudgetSource, Category, CategoryKind, DepositMode, Entry,
    EntryKind, Frequency, IncomeOverride, Job, Member, OffsetRule, PayMode, Share,
};
use jizifin_domain::ids::{
    AccountId, CategoryId, EntryId, JobId, MemberId, OffsetRuleId, ProjectId, TagId,
};
use jizifin_domain::{Cents, Ciphertext, DomainError, Month};

type R<T> = duckdb::Result<T>;

fn domain<T>(idx: usize, parsed: Result<T, DomainError>) -> R<T> {
    parsed.map_err(|e| duckdb::Error::FromSqlConversionFailure(idx, Type::Text, Box::new(e)))
}

fn text<T>(row: &Row<'_>, idx: usize, parse: fn(&str) -> Result<T, DomainError>) -> R<T> {
    let raw: String = row.get(idx)?;
    domain(idx, parse(&raw))
}

fn cents(row: &Row<'_>, idx: usize) -> R<Cents> {
    Ok(Cents::new(row.get(idx)?))
}

fn opt_id<T>(row: &Row<'_>, idx: usize, wrap: fn(i64) -> T) -> R<Option<T>> {
    Ok(row.get::<_, Option<i64>>(idx)?.map(wrap))
}

/// `id, name_ct, color, is_active`
pub fn member(row: &Row<'_>) -> R<Member> {
    let name = domain(1, Ciphertext::parse(row.get::<_, String>(1)?))?;
    Ok(Member { id: MemberId(row.get(0)?), name, color: row.get(2)?, is_active: row.get(3)? })
}

/// `id, kind, name_ct, pay_mode`
pub fn category(row: &Row<'_>) -> R<Category> {
    let name = domain(2, Ciphertext::parse(row.get::<_, String>(2)?))?;
    let (kind, pay_mode) = (text(row, 1, CategoryKind::parse)?, text(row, 3, PayMode::parse)?);
    Ok(Category { id: CategoryId(row.get(0)?), kind, name, pay_mode })
}

/// `member_id, pct`
pub fn share(row: &Row<'_>, offset: usize) -> R<Share> {
    Ok(Share { member: MemberId(row.get(offset)?), pct: row.get(offset + 1)? })
}

/// `id, kind, entry_date, amount_cents, member_id, category_id, account_id, project_id, tag_id`
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
    })
}

/// `id, member_id, amount_cents, frequency, valid_from, valid_to`
pub fn job(row: &Row<'_>) -> R<Job> {
    Ok(Job {
        id: JobId(row.get(0)?),
        member: MemberId(row.get(1)?),
        amount: cents(row, 2)?,
        frequency: text(row, 3, Frequency::parse)?,
        valid_from: row.get(4)?,
        valid_to: row.get(5)?,
    })
}

/// `member_id, month, amount_cents`
pub fn income_override(row: &Row<'_>) -> R<IncomeOverride> {
    Ok(IncomeOverride { member: MemberId(row.get(0)?), month: Month::of(row.get(1)?), amount: cents(row, 2)? })
}

/// `id, safety_margin_pct, deposit_mode` (members and categories attached later)
pub fn account(row: &Row<'_>) -> R<Account> {
    Ok(Account {
        id: AccountId(row.get(0)?),
        safety_margin_pct: row.get(1)?,
        deposit_mode: text(row, 2, DepositMode::parse)?,
        members: Vec::new(),
        categories: Vec::new(),
    })
}

/// `account_id, member_id, fixed_deposit_cents, due_day`
pub fn account_member(row: &Row<'_>) -> R<(AccountId, AccountMember)> {
    let fixed_deposit = row.get::<_, Option<i64>>(2)?.map(Cents::new);
    let member = AccountMember { member: MemberId(row.get(1)?), fixed_deposit, due_day: row.get(3)? };
    Ok((AccountId(row.get(0)?), member))
}

/// `account_id, category_id, expected_cents`
pub fn account_category(row: &Row<'_>) -> R<(AccountId, AccountCategory)> {
    let category = AccountCategory { category: CategoryId(row.get(1)?), expected: cents(row, 2)? };
    Ok((AccountId(row.get(0)?), category))
}

/// `id, credit_member_id, credit_category_id, debit_member_id, debit_category_id`
pub fn offset_rule(row: &Row<'_>) -> R<OffsetRule> {
    Ok(OffsetRule {
        id: OffsetRuleId(row.get(0)?),
        credit_member: MemberId(row.get(1)?),
        credit_category: CategoryId(row.get(2)?),
        debit_member: MemberId(row.get(3)?),
        debit_category: CategoryId(row.get(4)?),
    })
}

/// `category_id, month, limit_cents, source`
pub fn budget(row: &Row<'_>) -> R<Budget> {
    Ok(Budget {
        category: CategoryId(row.get(0)?),
        month: Month::of(row.get(1)?),
        limit: cents(row, 2)?,
        source: text(row, 3, BudgetSource::parse)?,
    })
}
