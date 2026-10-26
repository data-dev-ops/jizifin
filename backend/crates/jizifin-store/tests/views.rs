use chrono::NaiveDate;
use duckdb::Transaction;
use jizifin_domain::entities::{CategoryKind, DepositMode, EntryKind, PayMode};
use jizifin_domain::{Cents, Ciphertext, Month};
use jizifin_store::inputs::{AccountInput, CategoryInput, EntryInput, MemberInput};
use jizifin_store::repo::{Accounts, Categories, Entries, Focus, Members, MonthSlice, Views};
use jizifin_store::{Database, StoreError};
use std::collections::BTreeSet;

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn populate_month(tx: &Transaction) -> Result<(), StoreError> {
    let m1 = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
    let m2 = Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })?;
    let c1 = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Food"), pay_mode: PayMode::Shared })?;
    let c2 = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Utilities"), pay_mode: PayMode::Shared })?;

    Entries::create(tx, &EntryInput {
        kind: EntryKind::Expense, date: d(2026, 7, 5), amount: Cents::new(6000),
        member: Some(m1), category: Some(c1), account: None, project: None, tag: None,
        name: Some(ct("Groceries")), note: None, overrides: Vec::new(),
    })?;
    Entries::create(tx, &EntryInput {
        kind: EntryKind::Expense, date: d(2026, 7, 10), amount: Cents::new(4000),
        member: Some(m2), category: Some(c2), account: None, project: None, tag: None,
        name: Some(ct("Electricity")), note: None, overrides: Vec::new(),
    })?;
    Ok(())
}

#[test]
fn views_month_totals_by_category_and_member() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        populate_month(tx)?;
        let month = Month::parse("2026-07").unwrap();
        let slice = MonthSlice { kind: EntryKind::Expense, month, members: BTreeSet::new(), focus: Focus::default() };
        let view = Views::month(tx, &slice)?;
        assert_eq!(view.total, Cents::new(10000));
        assert_eq!(view.count, 2);
        assert_eq!(view.average, Some(Cents::new(5000)));
        assert_eq!(view.by_category[0].total, Cents::new(6000));
        assert_eq!(view.by_category[0].share_pct, Some(60.0));
        assert_eq!(view.by_member[0].total, Cents::new(6000));
        assert_eq!(view.by_member[0].share_pct, Some(60.0));
        Ok(())
    }).unwrap();
}

#[test]
fn views_series_aggregation() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Food"), pay_mode: PayMode::Shared })?;
        for (day, amt) in [(5, 3000), (20, 2000)] {
            Entries::create(tx, &EntryInput {
                kind: EntryKind::Expense, date: d(2026, 7, day), amount: Cents::new(amt),
                member: Some(m), category: Some(c), account: None, project: None, tag: None,
                name: Some(ct("G")), note: None, overrides: Vec::new(),
            })?;
        }
        let p = Month::parse("2026-07").unwrap().period();
        let series = Views::series(tx, EntryKind::Expense, p)?;
        assert_eq!(series.len(), 1);
        assert_eq!(series[0].total, Cents::new(5000));
        Ok(())
    }).unwrap();
}

#[test]
fn month_view_of_entries_without_category_does_not_fail() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let account = Accounts::create(tx, &AccountInput {
            name: ct("Pool"), safety_margin_pct: 0.0, expected_override: None, deposit_mode: DepositMode::Equal,
            members: Vec::new(), categories: Vec::new(),
        })?;
        Entries::create(tx, &EntryInput {
            kind: EntryKind::Deposit, date: d(2026, 7, 5), amount: Cents::new(2500), member: Some(m),
            category: None, account: Some(account), project: None, tag: None, name: None, note: None, overrides: Vec::new(),
        })?;
        let slice = MonthSlice { kind: EntryKind::Deposit, month: Month::parse("2026-07").unwrap(), members: BTreeSet::new(), focus: Focus::default() };
        let view = Views::month(tx, &slice)?;
        assert_eq!(view.total, Cents::new(2500));
        assert_eq!(view.by_category[0].key, 0);
        Ok(())
    }).unwrap();
}
