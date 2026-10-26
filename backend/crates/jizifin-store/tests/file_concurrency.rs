use chrono::NaiveDate;
use jizifin_domain::entities::{CategoryKind, EntryKind, PayMode};
use jizifin_domain::ids::{CategoryId, MemberId};
use jizifin_domain::{Cents, Ciphertext};
use jizifin_store::inputs::{CategoryInput, EntryInput, MemberInput};
use jizifin_store::repo::{Categories, Entries, Members};
use jizifin_store::Database;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn seed_fixture(db: &Database) -> (MemberId, CategoryId) {
    let m = db.write(|tx| Members::create(tx, &MemberInput { name: ct("Alice"), color: "#111".into(), is_active: true })).unwrap();
    let c = db.write(|tx| Categories::create(tx, &CategoryInput { kind: CategoryKind::Expense, name: ct("Groceries"), pay_mode: PayMode::Shared })).unwrap();
    (m, c)
}

fn write_worker(db: &Database, m: MemberId, c: CategoryId, worker_id: usize) {
    for i in 0..5 {
        db.write(|tx| {
            Entries::create(tx, &EntryInput {
                kind: EntryKind::Expense,
                date: NaiveDate::from_ymd_opt(2026, 7, 1 + (i as u32)).unwrap(),
                amount: Cents::new(1000 * ((worker_id as i64) + 1)),
                member: Some(m), category: Some(c), account: None, project: None, tag: None,
                name: Some(ct(&format!("Item_{worker_id}_{i}"))),
                note: None, overrides: Vec::new(),
            })
        }).unwrap();
    }
}

fn read_worker(db: &Database, reads: &AtomicUsize) {
    for _ in 0..10 {
        let entries = db.read(|conn| Entries::list(conn, &Default::default())).unwrap();
        let members = db.read(Members::list).unwrap();
        assert!(!members.is_empty());
        let _ = entries.entries.len();
        reads.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn multi_threaded_file_backed_write_serialization_and_concurrency() {
    let path = std::env::temp_dir().join(format!("concurrency_test_{}.duckdb", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let db = Database::open(&path).unwrap();
    let (m, c) = seed_fixture(&db);
    let initial_version = db.version().current();
    let reads = Arc::new(AtomicUsize::new(0));

    std::thread::scope(|s| {
        for id in 0..4 {
            let (db, reads) = (&db, &reads);
            s.spawn(move || write_worker(db, m, c, id));
            s.spawn(move || read_worker(db, reads));
        }
    });

    let count = db.read(|conn| Ok(Entries::list(conn, &Default::default())?.entries.len())).unwrap();
    assert_eq!(count, 20);
    assert_eq!(db.version().current(), initial_version + 20);
    assert!(reads.load(Ordering::Relaxed) >= 40);
    let _ = std::fs::remove_file(&path);
}
