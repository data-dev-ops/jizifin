use chrono::NaiveDate;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode};
use jizifin_domain::ids::{CategoryId, MemberId};
use jizifin_domain::{Cents, Ciphertext};
use jizifin_store::inputs::{CategoryInput, EntryInput, MemberInput};
use jizifin_store::repo::{Categories, Entries, Members};
use jizifin_store::{Backup, Database, StoreError};

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn populate(db: &Database) -> (MemberId, CategoryId) {
    let m = db.write(|tx| Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })).unwrap();
    let c = db.write(|tx| Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Food"), pay_mode: PayMode::Shared })).unwrap();
    db.write(|tx| {
        Entries::create(tx, &EntryInput {
            kind: EntryKind::Expense, date: d(2026, 7, 10), amount: Cents::new(5000),
            member: Some(m), category: Some(c), account: None, project: None, tag: None,
            name: Some(ct("Dinner")), note: None, overrides: Vec::new(),
        })
    }).unwrap();
    (m, c)
}

fn verify_restored(db: &Database, m: MemberId, c: CategoryId) {
    let members = db.read(Members::list).unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].id, m);

    let cats = db.read(Categories::list).unwrap();
    assert_eq!(cats.len(), 1);
    assert_eq!(cats[0].id, c);

    let m2 = db.write(|tx| Members::create(tx, &MemberInput { name: ct("Bob"), color: "#222".into(), is_active: true })).unwrap();
    assert!(m2.0 > m.0);
}

#[test]
fn backup_export_and_restore_roundtrip_with_sequence_continuation() {
    let path = std::env::temp_dir().join(format!("test_backup_{}.duckdb", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let db1 = Database::in_memory().unwrap();
    let (m1, c1) = populate(&db1);
    Backup::export(&db1, &path).unwrap();

    let db2 = Database::in_memory().unwrap();
    Backup::restore(&db2, &path).unwrap();
    verify_restored(&db2, m1, c1);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn restore_rejects_corrupted_payload() {
    let path = std::env::temp_dir().join(format!("test_corrupt_{}.duckdb", std::process::id()));
    std::fs::write(&path, b"not a valid duckdb file content").unwrap();
    let db = Database::in_memory().unwrap();
    let err = Backup::restore(&db, &path).unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn restore_rejects_schema_version_mismatch() {
    let path = std::env::temp_dir().join(format!("test_v_mismatch_{}.duckdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let conn = duckdb::Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE schema_version (version BIGINT); INSERT INTO schema_version VALUES (999);").unwrap();
    drop(conn);
    let db = Database::in_memory().unwrap();
    let err = Backup::restore(&db, &path).unwrap_err();
    assert!(matches!(err, StoreError::VersionMismatch { found: 999, expected: _ }));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn restore_rejects_empty_duckdb_missing_schema() {
    let path = std::env::temp_dir().join(format!("test_empty_{}.duckdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let conn = duckdb::Connection::open(&path).unwrap();
    drop(conn);
    let db = Database::in_memory().unwrap();
    let err = Backup::restore(&db, &path).unwrap_err();
    assert!(matches!(err, StoreError::Invalid(_)));
    let _ = std::fs::remove_file(&path);
}

