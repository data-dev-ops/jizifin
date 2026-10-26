use chrono::NaiveDate;
use duckdb::Transaction;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode, Recurrence};
use jizifin_domain::ids::{CategoryId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};
use jizifin_store::inputs::{CategoryInput, MemberInput, RecurringInput};
use jizifin_store::repo::{Categories, Entries, Members, Recurring, Seals};
use jizifin_store::{Database, StoreError};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn setup_template(tx: &Transaction) -> Result<(), StoreError> {
    let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
    let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Internet"), pay_mode: PayMode::Shared })?;
    Recurring::create(tx, &RecurringInput {
        kind: EntryKind::Expense, amount: Cents::new(5000), member: m, category: c,
        account: None, day_of_month: 15, recurrence: Recurrence::Monthly,
        valid_from: d(2026, 1, 1), valid_to: None, name: ct("Monthly_Internet"), is_active: true,
    })?;
    Ok(())
}

fn setup_day31_template(tx: &Transaction, start: NaiveDate) -> Result<(MemberId, CategoryId), StoreError> {
    let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
    let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Rent"), pay_mode: PayMode::Shared })?;
    Recurring::create(tx, &RecurringInput {
        kind: EntryKind::Expense, amount: Cents::new(10000), member: m, category: c,
        account: None, day_of_month: 31, recurrence: Recurrence::Monthly,
        valid_from: start, valid_to: None, name: ct("MonthEnd_Rent"), is_active: true,
    })?;
    Ok((m, c))
}

#[test]
fn recurring_materialization_is_idempotent() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        setup_template(tx)?;
        // A never-run template backfills from its valid_from: Jan 15 .. Jul 15.
        let first = Recurring::materialize(tx, d(2026, 7, 20))?;
        assert_eq!(first.len(), 7);
        let second = Recurring::materialize(tx, d(2026, 7, 20))?;
        assert_eq!(second.len(), 0);
        let third = Recurring::materialize(tx, d(2026, 7, 25))?;
        assert_eq!(third.len(), 0);
        Ok(())
    }).unwrap();
}

#[test]
fn recurring_month_transition_catches_up_previous_month() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Bill"), pay_mode: PayMode::Shared })?;
        Recurring::create(tx, &RecurringInput {
            kind: EntryKind::Expense, amount: Cents::new(3000), member: m, category: c,
            account: None, day_of_month: 28, recurrence: Recurrence::Monthly,
            valid_from: d(2026, 2, 1), valid_to: None, name: ct("Feb_Bill"), is_active: true,
        })?;
        let created = Recurring::materialize(tx, d(2026, 3, 3))?;
        assert_eq!(created.len(), 1);
        let page = Entries::list(tx, &Default::default())?;
        assert_eq!(page.entries[0].entry.date, d(2026, 2, 28));
        Ok(())
    }).unwrap();
}

#[test]
fn recurring_leap_year_clamp_february() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        setup_day31_template(tx, d(2024, 1, 1))?;
        let leap_created = Recurring::materialize(tx, d(2024, 3, 2))?;
        assert_eq!(leap_created.len(), 2);
        let dates: Vec<_> = Entries::list(tx, &Default::default())?.entries.iter().map(|e| e.entry.date).collect();
        assert!(dates.contains(&d(2024, 2, 29)));
        Ok(())
    }).unwrap();

    let db2 = Database::in_memory().unwrap();
    db2.write(|tx| {
        setup_day31_template(tx, d(2025, 1, 1))?;
        let non_leap = Recurring::materialize(tx, d(2025, 3, 2))?;
        assert_eq!(non_leap.len(), 2);
        let dates: Vec<_> = Entries::list(tx, &Default::default())?.entries.iter().map(|e| e.entry.date).collect();
        assert!(dates.contains(&d(2025, 2, 28)));
        Ok(())
    }).unwrap();
}

#[test]
fn recurring_inactive_and_validity_window_ignored() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Old"), pay_mode: PayMode::Shared })?;
        Recurring::create(tx, &RecurringInput {
            kind: EntryKind::Expense, amount: Cents::new(1000), member: m, category: c,
            account: None, day_of_month: 10, recurrence: Recurrence::Monthly,
            valid_from: d(2026, 1, 1), valid_to: None, name: ct("Paused"), is_active: false,
        })?;
        Recurring::create(tx, &RecurringInput {
            kind: EntryKind::Expense, amount: Cents::new(2000), member: m, category: c,
            account: None, day_of_month: 10, recurrence: Recurrence::Monthly,
            valid_from: d(2026, 1, 1), valid_to: Some(d(2026, 1, 31)), name: ct("Expired"), is_active: true,
        })?;
        // Paused is ignored; Expired still owes its January occurrence, and nothing after valid_to.
        let created = Recurring::materialize(tx, d(2026, 2, 15))?;
        assert_eq!(created.len(), 1);
        Ok(())
    }).unwrap();
}

#[test]
fn recurring_skips_sealed_months_across_boundary() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        setup_day31_template(tx, d(2026, 1, 1))?;
        Seals::insert(tx, Month::parse("2026-02").unwrap(), "{}", "hash")?;
        // January is open and materializes; sealed February is skipped.
        let created = Recurring::materialize(tx, d(2026, 3, 3))?;
        assert_eq!(created.len(), 1);
        Ok(())
    }).unwrap();
}


#[test]
fn recurring_catches_up_after_long_downtime() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        setup_day31_template(tx, d(2026, 1, 1))?;
        assert_eq!(Recurring::materialize(tx, d(2026, 1, 31))?.len(), 1);
        // Backend offline for months: every missed occurrence is recovered, not only the last week's.
        let created = Recurring::materialize(tx, d(2026, 6, 15))?;
        assert_eq!(created.len(), 4);
        Ok(())
    }).unwrap();
}
