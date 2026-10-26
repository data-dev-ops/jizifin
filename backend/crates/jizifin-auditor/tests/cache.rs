use chrono::NaiveDate;
use jizifin_auditor::cache::{DataVersion, SnapshotCache};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::Month;
use std::sync::Arc;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn data_version_monotonic_bump() {
    let ver = DataVersion::default();
    assert_eq!(ver.current(), 0);
    ver.bump();
    assert_eq!(ver.current(), 1);
    ver.bump();
    assert_eq!(ver.current(), 2);
}

#[test]
fn snapshot_cache_hit_and_invalidation() {
    let cache = SnapshotCache::new(2);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let snap = Arc::new(Snapshot::default());

    // Miss on empty cache
    assert!(cache.get(&scope, 1).is_none());

    // Put into cache
    cache.put(scope.clone(), 1, Arc::clone(&snap));
    assert!(cache.get(&scope, 1).is_some());

    // Bump version -> old version misses
    assert!(cache.get(&scope, 2).is_none());

    // Putting version 2 clears version 1
    let snap2 = Arc::new(Snapshot::default());
    cache.put(scope.clone(), 2, snap2);
    assert!(cache.get(&scope, 1).is_none());
    assert!(cache.get(&scope, 2).is_some());
}

#[test]
fn snapshot_cache_lru_capacity_eviction() {
    let cache = SnapshotCache::new(2);
    let s1 = AuditScope::month(Month::parse("2026-05").unwrap(), d(2026, 5, 31));
    let s2 = AuditScope::month(Month::parse("2026-06").unwrap(), d(2026, 6, 30));
    let s3 = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));

    cache.put(s1.clone(), 1, Arc::new(Snapshot::default()));
    cache.put(s2.clone(), 1, Arc::new(Snapshot::default()));
    assert!(cache.get(&s1, 1).is_some());

    // Adding s3 evicts s2 (since s1 was accessed above)
    cache.put(s3.clone(), 1, Arc::new(Snapshot::default()));
    assert!(cache.get(&s1, 1).is_some());
    assert!(cache.get(&s3, 1).is_some());
    assert!(cache.get(&s2, 1).is_none());
}

#[test]
fn snapshot_inputs_hash_is_deterministic() {
    let snap1 = Snapshot::default();
    let snap2 = Snapshot::default();
    assert_eq!(snap1.inputs_hash(), snap2.inputs_hash());
    assert!(!snap1.inputs_hash().is_empty());
}
