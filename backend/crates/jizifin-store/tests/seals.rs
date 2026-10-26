use chrono::NaiveDate;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode};
use jizifin_domain::ids::{CategoryId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};
use jizifin_store::inputs::{CategoryInput, EntryInput, MemberInput};
use jizifin_store::repo::{Categories, Entries, Members, Seals};
use jizifin_store::{Database, StoreError};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn make_entry(member: MemberId, category: CategoryId, date: NaiveDate) -> EntryInput {
    EntryInput {
        kind: EntryKind::Expense,
        date,
        amount: Cents::new(5000),
        member: Some(member),
        category: Some(category),
        account: None,
        project: None,
        tag: None,
        name: Some(ct("Expense")),
        note: None,
        overrides: Vec::new(),
    }
}

#[test]
fn month_sealing_lifecycle_and_conflict_on_duplicate() {
    let db = Database::in_memory().unwrap();
    let month = Month::parse("2026-05").unwrap();
    db.write(|tx| {
        Seals::insert(tx, month, r#"{"status":"sealed"}"#, "dummyhash")?;
        let seal = Seals::get(tx, month)?;
        assert!(seal.is_some());
        assert_eq!(seal.unwrap().inputs_hash, "dummyhash");

        // Duplicate seal of same month fails with Conflict
        let dup = Seals::insert(tx, month, r#"{"status":"sealed"}"#, "hash2");
        assert!(matches!(dup, Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn sealed_month_blocks_mutations() {
    let db = Database::in_memory().unwrap();
    let month = Month::parse("2026-06").unwrap();
    db.write(|tx| {
        let m_id = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let c_id = Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Groceries"), pay_mode: PayMode::Shared })?;

        // Entry created BEFORE sealing succeeds
        let entry_input = make_entry(m_id, c_id, d(2026, 6, 15));
        let entry_id = Entries::create(tx, &entry_input)?;

        // Seal June 2026
        Seals::insert(tx, month, r#"{"status":"ok"}"#, "h1")?;

        // 1. Post new entry in June 2026 fails with Conflict
        let post_err = Entries::create(tx, &entry_input);
        assert!(matches!(post_err, Err(StoreError::Conflict(_))));

        // 2. Update existing entry in June 2026 fails with Conflict
        let mut updated = make_entry(m_id, c_id, d(2026, 6, 15));
        updated.amount = Cents::new(9999);
        let update_err = Entries::update(tx, entry_id, &updated);
        assert!(matches!(update_err, Err(StoreError::Conflict(_))));

        // 3. Delete existing entry in June 2026 fails with Conflict
        let del_err = Entries::delete(tx, entry_id);
        assert!(matches!(del_err, Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

fn job_input(member: MemberId, amount: i64, valid_to: Option<NaiveDate>) -> jizifin_store::inputs::JobInput {
    jizifin_store::inputs::JobInput {
        member, name: ct("Employer"), amount: Cents::new(amount), frequency: jizifin_domain::entities::Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to, notes: None,
    }
}

#[test]
fn sealed_month_protects_jobs() {
    use jizifin_store::repo::Jobs;
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        let job = Jobs::create(tx, &job_input(m, 300000, None))?;
        Seals::insert(tx, Month::parse("2026-03").unwrap(), "{}", "hash")?;

        // A retroactive raise touches sealed March.
        assert!(matches!(Jobs::update(tx, job, &job_input(m, 400000, None)), Err(StoreError::Conflict(_))));
        // A name-only edit changes no money.
        Jobs::update(tx, job, &jizifin_store::inputs::JobInput { name: ct("Renamed"), ..job_input(m, 300000, None) })?;
        // Ending the contract after the sealed month is fine; ending it inside it is not.
        assert!(matches!(Jobs::update(tx, job, &job_input(m, 300000, Some(d(2026, 3, 10)))), Err(StoreError::Conflict(_))));
        Jobs::update(tx, job, &job_input(m, 300000, Some(d(2026, 6, 30))))?;
        assert!(matches!(Jobs::delete(tx, job), Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn sealed_month_protects_income_overrides() {
    use jizifin_domain::entities::IncomeOverride;
    use jizifin_store::repo::IncomeOverrides;
    let db = Database::in_memory().unwrap();
    db.write(|tx| {
        let m = Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })?;
        Seals::insert(tx, Month::parse("2026-03").unwrap(), "{}", "hash")?;
        let march = IncomeOverride { member: m, month: Month::parse("2026-03").unwrap(), amount: Cents::new(1), note: None };
        assert!(matches!(IncomeOverrides::upsert(tx, &march), Err(StoreError::Conflict(_))));
        Ok(())
    }).unwrap();
}

#[test]
fn unsealing_reopens_the_month() {
    let db = Database::in_memory().unwrap();
    let month = Month::parse("2026-04").unwrap();
    db.write(|tx| {
        Seals::insert(tx, month, "{}", "hash")?;
        Seals::delete(tx, month)?;
        assert!(Seals::get(tx, month)?.is_none());
        assert!(matches!(Seals::delete(tx, month), Err(StoreError::NotFound(_))));
        Ok(())
    }).unwrap();
}
