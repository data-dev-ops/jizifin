use jizifin_auditor::allocation::{LargestRemainderAllocator, TieSeed, Weight};
use jizifin_domain::ids::MemberId;

fn w(member: i64, weight: f64) -> Weight {
    Weight { member: MemberId(member), weight }
}

#[test]
fn allocate_exact_division() {
    let weights = vec![w(1, 50.0), w(2, 50.0)];
    let seed = TieSeed::named("exact");
    let alloc = LargestRemainderAllocator.allocate(1000, &weights, &seed);
    assert_eq!(alloc.total(), 1000);
    assert_eq!(alloc.get(MemberId(1)), 500);
    assert_eq!(alloc.get(MemberId(2)), 500);
}

#[test]
fn allocate_odd_cent_split() {
    let weights = vec![w(1, 1.0), w(2, 1.0), w(3, 1.0)];
    let seed = TieSeed::named("odd");
    let alloc = LargestRemainderAllocator.allocate(100, &weights, &seed);
    assert_eq!(alloc.total(), 100);
    let shares: Vec<i64> = alloc.iter().map(|(_, u)| u).collect();
    assert_eq!(shares.iter().sum::<i64>(), 100);
    assert!(shares.contains(&34));
    assert_eq!(shares.iter().filter(|&&s| s == 33).count(), 2);
}

#[test]
fn allocate_refund_negative_total() {
    let weights = vec![w(1, 1.0), w(2, 1.0), w(3, 1.0)];
    let seed = TieSeed::named("refund");
    let alloc = LargestRemainderAllocator.allocate(-100, &weights, &seed);
    assert_eq!(alloc.total(), -100);
    let shares: Vec<i64> = alloc.iter().map(|(_, u)| u).collect();
    assert_eq!(shares.iter().sum::<i64>(), -100);
    assert!(shares.iter().all(|&s| s <= 0));
}

#[test]
fn allocate_zero_total_and_zero_weights() {
    let weights = vec![w(1, 0.0), w(2, 0.0)];
    let seed = TieSeed::named("zero");
    let alloc = LargestRemainderAllocator.allocate(500, &weights, &seed);
    assert_eq!(alloc.total(), 0);
    assert_eq!(alloc.get(MemberId(1)), 0);

    let weights2 = vec![w(1, 1.0)];
    let alloc2 = LargestRemainderAllocator.allocate(0, &weights2, &seed);
    assert_eq!(alloc2.total(), 0);
}

#[test]
fn allocate_single_cent_and_large_int() {
    let weights = vec![w(1, 1.0), w(2, 1.0), w(3, 1.0)];
    let seed = TieSeed::named("cent");
    let alloc = LargestRemainderAllocator.allocate(1, &weights, &seed);
    assert_eq!(alloc.total(), 1);

    let big = 9_007_199_254_740_991_i64;
    let alloc_big = LargestRemainderAllocator.allocate(big, &weights, &seed);
    assert_eq!(alloc_big.total(), big);
}

#[test]
fn deterministic_tie_break_across_runs() {
    let weights = vec![w(1, 50.0), w(2, 50.0)];
    let seed_a = TieSeed::named("test-seed-a");
    let seed_b = TieSeed::named("test-seed-b");
    let alloc_a1 = LargestRemainderAllocator.allocate(1, &weights, &seed_a);
    let alloc_a2 = LargestRemainderAllocator.allocate(1, &weights, &seed_a);
    assert_eq!(alloc_a1, alloc_a2);
    let _alloc_b = LargestRemainderAllocator.allocate(1, &weights, &seed_b);
}

#[test]
fn allocate_negative_and_zero_weights() {
    let weights = vec![w(1, -5.0), w(2, 50.0)];
    let seed = TieSeed::named("neg");
    let alloc = LargestRemainderAllocator.allocate(1000, &weights, &seed);
    assert_eq!(alloc.get(MemberId(1)), 0);
    assert_eq!(alloc.get(MemberId(2)), 1000);

    let all_neg = vec![w(1, -5.0), w(2, -10.0)];
    let alloc_neg = LargestRemainderAllocator.allocate(1000, &all_neg, &seed);
    assert_eq!(alloc_neg.total(), 0);
}

#[test]
fn allocate_empty_weights_returns_empty() {
    let seed = TieSeed::named("empty");
    let alloc = LargestRemainderAllocator.allocate(100, &[], &seed);
    assert_eq!(alloc.total(), 0);
}

#[test]
fn tie_seed_entry_and_named() {
    let weights = vec![w(1, 50.0), w(2, 50.0)];
    let seed_entry = TieSeed::entry(jizifin_domain::ids::EntryId(42));
    let alloc1 = LargestRemainderAllocator.allocate(1, &weights, &seed_entry);
    let alloc2 = LargestRemainderAllocator.allocate(1, &weights, &seed_entry);
    assert_eq!(alloc1, alloc2);
    assert_eq!(alloc1.total(), 1);
}

#[test]
fn allocation_accessors_and_iter() {
    let weights = vec![w(1, 50.0), w(2, 50.0)];
    let seed = TieSeed::named("iter");
    let alloc = LargestRemainderAllocator.allocate(1000, &weights, &seed);
    assert_eq!(alloc.get(MemberId(999)), 0);
    let items: Vec<(MemberId, i64)> = alloc.iter().collect();
    assert_eq!(items, vec![(MemberId(1), 500), (MemberId(2), 500)]);
}
