use chrono::NaiveDate;
use jizifin_domain::entities::{AccountCategory, AccountMember, BudgetSource, CategoryKind, DepositMode, EntryKind, PayMode, Share};
use jizifin_domain::ids::{CategoryId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};
use jizifin_store::inputs::{AccountInput, BudgetInput, CategoryInput, EntryInput, MemberInput};
use jizifin_store::repo::{Accounts, Budgets, Categories, Entries, Members};
use jizifin_store::{Database, StoreError};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn make_entry(member: Option<MemberId>, category: Option<CategoryId>, kind: EntryKind) -> EntryInput {
    EntryInput {
        kind,
        date: d(2026, 7, 15),
        amount: Cents::new(5000),
        member,
        category,
        account: None,
        project: None,
        tag: None,
        name: Some(ct("Item")),
        note: None,
        overrides: Vec::new(),
    }
}

#[test]
fn missing_member_or_category_fails_integrity() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let entry = make_entry(Some(MemberId(999)), Some(CategoryId(999)), EntryKind::Expense);
        let res = Entries::create(tx, &entry);
        assert!(matches!(res, Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn category_kind_mismatch_rejected() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m_id = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c_id = Categories::create(tx, &CategoryInput { kind: CategoryKind::Income, name: ct("Salary"), pay_mode: PayMode::Shared })?;
        // Attempting to post an Expense with an Income category
        let entry = make_entry(Some(m_id), Some(c_id), EntryKind::Expense);
        let res = Entries::create(tx, &entry);
        assert!(matches!(res, Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn cannot_delete_referenced_member_or_category() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m_id = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c_id = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Groceries"), pay_mode: PayMode::Shared })?;
        let entry = make_entry(Some(m_id), Some(c_id), EntryKind::Expense);
        let _e_id = Entries::create(tx, &entry)?;

        // Deleting category while referenced by entry fails with Conflict
        let del_cat = Categories::delete(tx, c_id);
        assert!(matches!(del_cat, Err(StoreError::Conflict(_))));

        // Deleting member while referenced by entry fails with Conflict
        let del_mem = Members::delete(tx, m_id);
        assert!(matches!(del_mem, Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn budget_income_category_rejected_and_cascade_delete() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let inc_cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Income, name: ct("Salary"), pay_mode: PayMode::Shared })?;
        let exp_cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Groceries"), pay_mode: PayMode::Shared })?;
        let bad_b = BudgetInput { category: inc_cat, valid_from: Month::parse("2026-01").unwrap(), valid_to: None, limit: Cents::new(50000), source: BudgetSource::Manual };
        assert!(matches!(Budgets::create(tx, &bad_b), Err(StoreError::Invalid(_))));
        let good_b = BudgetInput { category: exp_cat, valid_from: Month::parse("2026-01").unwrap(), valid_to: None, limit: Cents::new(50000), source: BudgetSource::Manual };
        Budgets::create(tx, &good_b)?;
        Categories::delete(tx, exp_cat)?;
        assert!(Budgets::all(tx)?.is_empty());
        Ok(())
    }).unwrap();
}

#[test]
fn joint_account_missing_member_or_category_rejected() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let acc_bad_mem = AccountInput {
            name: ct("Joint"), safety_margin_pct: 10.0, expected_override: None, deposit_mode: DepositMode::Equal,
            members: vec![AccountMember { member: MemberId(999), fixed_deposit: None, due_day: 5 }], categories: vec![],
        };
        assert!(matches!(Accounts::create(tx, &acc_bad_mem), Err(StoreError::Invalid(_))));
        let acc_bad_cat = AccountInput {
            name: ct("Joint"), safety_margin_pct: 10.0, expected_override: None, deposit_mode: DepositMode::Equal,
            members: vec![], categories: vec![AccountCategory { category: CategoryId(999), expected: Cents::new(1000) }],
        };
        assert!(matches!(Accounts::create(tx, &acc_bad_cat), Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn entry_share_overrides_delete_referenced_member_conflict() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m1 = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let m2 = Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })?;
        let cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Rent"), pay_mode: PayMode::Shared })?;
        let mut entry = make_entry(Some(m1), Some(cat), EntryKind::Expense);
        entry.overrides = vec![Share { member: m1, pct: 60.0 }, Share { member: m2, pct: 40.0 }];
        Entries::create(tx, &entry)?;
        assert!(matches!(Members::delete(tx, m2), Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn entry_share_overrides_validation_pct_and_duplicates() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m1 = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let m2 = Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })?;
        let cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Rent"), pay_mode: PayMode::Shared })?;
        let mut entry = make_entry(Some(m1), Some(cat), EntryKind::Expense);
        entry.overrides = vec![Share { member: m1, pct: 40.0 }, Share { member: m2, pct: 40.0 }];
        assert!(matches!(Entries::create(tx, &entry), Err(StoreError::Invalid(_))));
        entry.overrides = vec![Share { member: m1, pct: 50.0 }, Share { member: m1, pct: 50.0 }];
        assert!(matches!(Entries::create(tx, &entry), Err(StoreError::Invalid(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn entry_delete_cascades_overrides_allowing_member_delete() {
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m1 = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let m2 = Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })?;
        let cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Rent"), pay_mode: PayMode::Shared })?;
        let mut entry = make_entry(Some(m1), Some(cat), EntryKind::Expense);
        entry.overrides = vec![Share { member: m1, pct: 50.0 }, Share { member: m2, pct: 50.0 }];
        let e_id = Entries::create(tx, &entry)?;
        assert!(matches!(Members::delete(tx, m2), Err(StoreError::Conflict(_))));
        Entries::delete(tx, e_id)?;
        assert!(Members::delete(tx, m2).is_ok());
        Ok(())
    }).unwrap();
}

fn seed_concurrent_members_and_cat(db: &Database) -> (MemberId, MemberId, CategoryId) {
    db.write(|tx| {
        let m1 = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let m2 = Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })?;
        let cat = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Rent"), pay_mode: PayMode::Shared })?;
        Ok((m1, m2, cat))
    }).unwrap()
}

fn write_override_entry(db: &Database, (m1, m2, cat): (MemberId, MemberId, CategoryId), amt: i64) {
    db.write(|tx| {
        let mut entry = make_entry(Some(m1), Some(cat), EntryKind::Expense);
        entry.amount = Cents::new(amt);
        entry.overrides = vec![Share { member: m1, pct: 50.0 }, Share { member: m2, pct: 50.0 }];
        Entries::create(tx, &entry)?;
        Ok(())
    }).unwrap();
}

#[test]
fn concurrent_entry_writes_with_overrides() {
    let db = Database::in_memory().unwrap();
    let ids = seed_concurrent_members_and_cat(&db);
    std::thread::scope(|s| {
        for i in 0..4 {
            let db = &db;
            s.spawn(move || write_override_entry(db, ids, 1000 * (i + 1)));
        }
    });
    let count = db.read(|conn| Ok(Entries::list(conn, &Default::default())?.entries.len())).unwrap();
    assert_eq!(count, 4);
}



