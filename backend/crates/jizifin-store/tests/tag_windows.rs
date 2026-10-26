use chrono::NaiveDate;
use duckdb::Transaction;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode};
use jizifin_domain::ids::{CategoryId, MemberId, TagId};
use jizifin_domain::{Cents, Ciphertext};
use jizifin_store::inputs::{CategoryInput, EntryInput, MemberInput, TagInput};
use jizifin_store::repo::{Categories, Entries, Members, Tags};
use jizifin_store::{Database, StoreError};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn test_entry(m: MemberId, c: CategoryId, tag: Option<TagId>, date: NaiveDate) -> EntryInput {
    EntryInput {
        kind: EntryKind::Expense, date, amount: Cents::new(1000), member: Some(m), category: Some(c),
        account: None, project: None, tag, name: Some(ct("Item")), note: None, overrides: Vec::new(),
    }
}

fn tag_input(name: &str, from: NaiveDate, to: NaiveDate) -> TagInput {
    TagInput {
        name: ct(name), description: None, color: "#f59e0b".into(),
        valid_from: Some(from), valid_to: Some(to),
    }
}

#[test]
fn tag_window_allows_inside_and_rejects_outside() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Travel"), pay_mode: PayMode::Shared })?;
        let tag = Tags::create(tx, &tag_input("Trip", d(2026, 7, 1), d(2026, 7, 10)))?;

        let e1 = test_entry(m, c, Some(tag), d(2026, 7, 5));
        assert!(Entries::create(tx, &e1).is_ok());

        let e2 = test_entry(m, c, Some(tag), d(2026, 7, 20));
        assert!(matches!(Entries::create(tx, &e2), Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

fn setup_tag_and_entry(tx: &Transaction) -> Result<TagId, StoreError> {
    let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
    let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Travel"), pay_mode: PayMode::Shared })?;
    let tag = Tags::create(tx, &tag_input("Trip", d(2026, 7, 1), d(2026, 7, 31)))?;
    let e1 = test_entry(m, c, Some(tag), d(2026, 7, 25));
    Entries::create(tx, &e1)?;
    Ok(tag)
}

#[test]
fn shrinking_tag_window_past_assigned_entry_fails() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let tag = setup_tag_and_entry(tx)?;
        let shrunk = tag_input("Trip", d(2026, 7, 1), d(2026, 7, 10));
        let res = Tags::update(tx, tag, &shrunk);
        assert!(matches!(res, Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn tag_window_exact_inclusive_bounds() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Travel"), pay_mode: PayMode::Shared })?;
        let tag = Tags::create(tx, &tag_input("Festival", d(2026, 7, 10), d(2026, 7, 15)))?;

        // Exactly on valid_from (July 10) -> OK
        assert!(Entries::create(tx, &test_entry(m, c, Some(tag), d(2026, 7, 10))).is_ok());
        // Exactly on valid_to (July 15) -> OK
        assert!(Entries::create(tx, &test_entry(m, c, Some(tag), d(2026, 7, 15))).is_ok());
        // One day before valid_from (July 9) -> Rejected
        assert!(matches!(Entries::create(tx, &test_entry(m, c, Some(tag), d(2026, 7, 9))), Err(StoreError::Invalid(_))));
        // One day after valid_to (July 16) -> Rejected
        assert!(matches!(Entries::create(tx, &test_entry(m, c, Some(tag), d(2026, 7, 16))), Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}
