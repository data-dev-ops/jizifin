use chrono::NaiveDate;
use jizifin_auditor::allocation::Weight;
use jizifin_auditor::shares::{EntryShares, ShareResolver, ShareSource};
use jizifin_auditor::Snapshot;
use jizifin_domain::entities::{Agreement, Category, CategoryKind, Entry, EntryKind, PayMode, Share};
use jizifin_domain::ids::{AgreementId, CategoryId, EntryId, MemberId};
use jizifin_domain::{Cents, Ciphertext};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn test_entry((id, cat, payer): (i64, i64, i64), amount: i64, date: NaiveDate) -> Entry {
    Entry {
        id: EntryId(id), kind: EntryKind::Expense, date, amount: Cents::new(amount),
        member: Some(MemberId(payer)), category: Some(CategoryId(cat)),
        account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Dinner")), note: None,
    }
}

fn setup_snap(mode: PayMode) -> (Snapshot, Entry) {
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Dining"), pay_mode: mode };
    let mut snap = Snapshot::default();
    snap.categories.insert(cat.id, cat);
    let entry = test_entry((100, 1, 1), 10000, d(2026, 7, 15));
    snap.entries.push(entry.clone());
    (snap, entry)
}

#[test]
fn personal_pay_mode_takes_precedence_over_all() {
    let (mut snap, entry) = setup_snap(PayMode::Personal);
    snap.overrides.insert(entry.id, vec![Share { member: MemberId(2), pct: 100.0 }]);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::Personal);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(10000)));
    assert_eq!(shares.shares.get(&MemberId(2)), None);
}

#[test]
fn override_takes_precedence_over_agreement_and_income() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.overrides.insert(entry.id, vec![
        Share { member: MemberId(1), pct: 80.0 }, Share { member: MemberId(2), pct: 20.0 },
    ]);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::Override);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(8000)));
    assert_eq!(shares.shares.get(&MemberId(2)).copied(), Some(Cents::new(2000)));
}

#[test]
fn income_ratio_precedence_when_income_present() {
    let (snap, entry) = setup_snap(PayMode::IncomeRatio);
    let weights = vec![
        Weight { member: MemberId(1), weight: 3000.0 }, Weight { member: MemberId(2), weight: 1000.0 },
    ];
    let resolver = ShareResolver::new(&snap, weights, vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::IncomeRatio);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(7500)));
    assert_eq!(shares.shares.get(&MemberId(2)).copied(), Some(Cents::new(2500)));
}

fn sample_agreements() -> Vec<Agreement> {
    vec![
        Agreement {
            id: AgreementId(1), category: CategoryId(1), valid_from: d(2020, 1, 1), valid_to: None, note: None,
            shares: vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(2), pct: 50.0 }],
        },
        Agreement {
            id: AgreementId(2), category: CategoryId(1), valid_from: d(2026, 7, 1), valid_to: Some(d(2026, 7, 31)), note: None,
            shares: vec![Share { member: MemberId(1), pct: 70.0 }, Share { member: MemberId(2), pct: 30.0 }],
        },
    ]
}

#[test]
fn agreement_precedence_bounded_beats_baseline() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.agreements = sample_agreements();
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::Agreement);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(7000)));
    assert_eq!(shares.shares.get(&MemberId(2)).copied(), Some(Cents::new(3000)));
}

#[test]
fn fallback_to_equal_when_no_rules_apply() {
    let (snap, entry) = setup_snap(PayMode::Shared);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::Equal);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(5000)));
    assert_eq!(shares.shares.get(&MemberId(2)).copied(), Some(Cents::new(5000)));
    let trail: Vec<String> = shares.trail().collect();
    assert_eq!(trail.len(), 2);
    assert!(trail[0].contains("via equal"));
}

#[test]
fn agreement_scd2_exact_boundary_dates() {
    let (mut snap, _) = setup_snap(PayMode::Shared);
    // Bounded agreement strictly from 2026-07-01 to 2026-07-15: 80/20
    snap.agreements = vec![
        Agreement {
            id: AgreementId(1), category: CategoryId(1), valid_from: d(2026, 7, 1), valid_to: Some(d(2026, 7, 15)), note: None,
            shares: vec![Share { member: MemberId(1), pct: 80.0 }, Share { member: MemberId(2), pct: 20.0 }],
        },
    ];
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);

    // Boundary 1: First day (2026-07-01) -> 80/20 agreement applies
    let e_start = test_entry((1, 1, 1), 10000, d(2026, 7, 1));
    let res_start = resolver.resolve(&e_start, MemberId(1), CategoryId(1));
    assert_eq!(res_start.source, ShareSource::Agreement);
    assert_eq!(EntryShares::new(&e_start, &res_start).shares[&MemberId(1)], Cents::new(8000));

    // Boundary 2: Last day (2026-07-15) -> 80/20 agreement applies
    let e_end = test_entry((2, 1, 1), 10000, d(2026, 7, 15));
    let res_end = resolver.resolve(&e_end, MemberId(1), CategoryId(1));
    assert_eq!(res_end.source, ShareSource::Agreement);
    assert_eq!(EntryShares::new(&e_end, &res_end).shares[&MemberId(1)], Cents::new(8000));

    // Boundary 3: Day after (2026-07-16) -> agreement expired, falls back to equal 50/50
    let e_after = test_entry((3, 1, 1), 10000, d(2026, 7, 16));
    let res_after = resolver.resolve(&e_after, MemberId(1), CategoryId(1));
    assert_eq!(res_after.source, ShareSource::Equal);
    assert_eq!(EntryShares::new(&e_after, &res_after).shares[&MemberId(1)], Cents::new(5000));
}

#[test]
fn inactive_member_excluded_from_equal_split_fallback() {
    let (snap, entry) = setup_snap(PayMode::Shared);
    // Active members M1 and M2; M3 is inactive and not passed in participants
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)).copied(), Some(Cents::new(5000)));
    assert_eq!(shares.shares.get(&MemberId(2)).copied(), Some(Cents::new(5000)));
    assert_eq!(shares.shares.get(&MemberId(3)), None);
}

#[test]
fn income_ratio_zero_income_falls_back_to_agreement_then_equal() {
    let (mut snap, entry) = setup_snap(PayMode::IncomeRatio);
    snap.agreements = vec![Agreement {
        id: AgreementId(1), category: CategoryId(1), valid_from: d(2026, 1, 1), valid_to: None, note: None,
        shares: vec![Share { member: MemberId(1), pct: 70.0 }, Share { member: MemberId(2), pct: 30.0 }],
    }];
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res.source, ShareSource::Agreement);

    snap.agreements.clear();
    let res_eq = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]).resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(res_eq.source, ShareSource::Equal);
}

#[test]
fn agreement_precedence_later_start_beats_earlier_start() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.agreements = vec![
        Agreement {
            id: AgreementId(1), category: CategoryId(1), valid_from: d(2026, 1, 1), valid_to: None, note: None,
            shares: vec![Share { member: MemberId(1), pct: 60.0 }, Share { member: MemberId(2), pct: 40.0 }],
        },
        Agreement {
            id: AgreementId(2), category: CategoryId(1), valid_from: d(2026, 6, 1), valid_to: None, note: None,
            shares: vec![Share { member: MemberId(1), pct: 80.0 }, Share { member: MemberId(2), pct: 20.0 }],
        },
    ];
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(EntryShares::new(&entry, &res).shares[&MemberId(1)], Cents::new(8000));
}

#[test]
fn agreement_precedence_higher_id_breaks_tie() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.agreements = vec![
        Agreement {
            id: AgreementId(10), category: CategoryId(1), valid_from: d(2026, 1, 1), valid_to: None, note: None,
            shares: vec![Share { member: MemberId(1), pct: 60.0 }, Share { member: MemberId(2), pct: 40.0 }],
        },
        Agreement {
            id: AgreementId(20), category: CategoryId(1), valid_from: d(2026, 1, 1), valid_to: None, note: None,
            shares: vec![Share { member: MemberId(1), pct: 90.0 }, Share { member: MemberId(2), pct: 10.0 }],
        },
    ];
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    assert_eq!(EntryShares::new(&entry, &res).shares[&MemberId(1)], Cents::new(9000));
}

#[test]
fn resolve_unpaid_treats_personal_mode_as_shared() {
    let (snap, entry) = setup_snap(PayMode::Personal);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve_unpaid(&entry, CategoryId(1));
    assert_eq!(res.source, ShareSource::Equal);
    assert_eq!(res.bucket.personal_of, None);
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares[&MemberId(1)], Cents::new(5000));
    assert_eq!(shares.shares[&MemberId(2)], Cents::new(5000));
}

#[test]
fn entry_share_overrides_three_members_asymmetric() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.overrides.insert(entry.id, vec![
        Share { member: MemberId(1), pct: 50.0 },
        Share { member: MemberId(2), pct: 30.0 },
        Share { member: MemberId(3), pct: 20.0 },
    ]);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2), MemberId(3)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares[&MemberId(1)], Cents::new(5000));
    assert_eq!(shares.shares[&MemberId(2)], Cents::new(3000));
    assert_eq!(shares.shares[&MemberId(3)], Cents::new(2000));
    assert_eq!(shares.shares_total(), Cents::new(10000));
}

#[test]
fn entry_share_overrides_full_liability_to_non_payer() {
    let (mut snap, entry) = setup_snap(PayMode::Shared);
    snap.overrides.insert(entry.id, vec![Share { member: MemberId(2), pct: 100.0 }]);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares.get(&MemberId(1)), None);
    assert_eq!(shares.shares[&MemberId(2)], Cents::new(10000));
}

#[test]
fn entry_shares_trail_and_refund() {
    let (mut snap, mut entry) = setup_snap(PayMode::Shared);
    entry.amount = Cents::new(-10000);
    snap.overrides.insert(entry.id, vec![
        Share { member: MemberId(1), pct: 50.0 },
        Share { member: MemberId(2), pct: 50.0 },
    ]);
    let resolver = ShareResolver::new(&snap, Vec::new(), vec![MemberId(1), MemberId(2)]);
    let res = resolver.resolve(&entry, MemberId(1), CategoryId(1));
    let shares = EntryShares::new(&entry, &res);
    assert_eq!(shares.shares[&MemberId(1)], Cents::new(-5000));
    assert_eq!(shares.shares_total(), Cents::new(-10000));
    let trail: Vec<String> = shares.trail().collect();
    assert_eq!(trail.len(), 2);
    assert!(trail[0].contains("via override"));
    assert!(trail[0].contains("-5000"));
}
