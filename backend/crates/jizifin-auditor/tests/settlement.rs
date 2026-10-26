use chrono::NaiveDate;
use jizifin_auditor::settlement::{SettlementCalculator, debts_settle, settlement_invariants};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::entities::{Agreement, Category, CategoryKind, Entry, EntryKind, Member, OffsetRule, PayMode, Share};
use jizifin_domain::ids::{AgreementId, CategoryId, EntryId, MemberId, OffsetRuleId};
use jizifin_domain::{Cents, Ciphertext, Month};
use std::collections::BTreeMap;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn add_member(snap: &mut Snapshot, id: i64) {
    snap.members.push(Member { id: MemberId(id), name: ct(&format!("M{id}")), color: "#000".into(), is_active: true });
}

fn add_category(snap: &mut Snapshot, id: i64, mode: PayMode) {
    snap.categories.insert(CategoryId(id), Category { id: CategoryId(id), kind: CategoryKind::Expense, name: ct(&format!("Cat{id}")), pay_mode: mode });
}

fn add_entry(snap: &mut Snapshot, id: i64, (cat, payer): (i64, i64), amount: i64) {
    snap.entries.push(Entry {
        id: EntryId(id), kind: EntryKind::Expense, date: d(2026, 7, 15), amount: Cents::new(amount),
        member: Some(MemberId(payer)), category: Some(CategoryId(cat)),
        account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Expense")), note: None,
    });
}

#[test]
fn simple_50_50_settlement() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].from, MemberId(2));
    assert_eq!(s.debts[0].to, MemberId(1));
    assert_eq!(s.debts[0].amount, Cents::new(5000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn asymmetric_60_40_settlement() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    snap.agreements.push(Agreement {
        id: AgreementId(1), category: CategoryId(1), valid_from: d(2026, 1, 1), valid_to: None, note: None,
        shares: vec![Share { member: MemberId(1), pct: 60.0 }, Share { member: MemberId(2), pct: 40.0 }],
    });
    add_entry(&mut snap, 1, (1, 1), 10000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].from, MemberId(2));
    assert_eq!(s.debts[0].to, MemberId(1));
    assert_eq!(s.debts[0].amount, Cents::new(4000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn connected_components_couples_isolation() {
    let mut snap = Snapshot::default();
    (1..=4).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    snap.overrides.insert(EntryId(1), vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(2), pct: 50.0 }]);
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(3), pct: 50.0 }, Share { member: MemberId(4), pct: 50.0 }]);
    add_entry(&mut snap, 1, (1, 1), 10000);
    add_entry(&mut snap, 2, (2, 3), 20000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.components.len(), 2);
    for debt in &s.debts {
        let same_comp = s.components.iter().any(|c| c.contains(&debt.from) && c.contains(&debt.to));
        assert!(same_comp, "Debt crossed connected components boundary!");
    }
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn zero_total_bucket_keeps_member_positions() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    // Shared category: M1 pays 100.00, M2 receives -100.00 refund -> bucket total is 0.00
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    add_entry(&mut snap, 2, (1, 2), -10000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::new(10000)));
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::new(-10000)));
    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].from, MemberId(2));
    assert_eq!(s.debts[0].to, MemberId(1));
    assert_eq!(s.debts[0].amount, Cents::new(10000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn offset_rule_transfers_debt() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    add_entry(&mut snap, 2, (2, 2), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(2), credit_category: CategoryId(2), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn debts_settle_verification_helper() {
    let mut net = BTreeMap::new();
    net.insert(MemberId(1), Cents::new(50));
    net.insert(MemberId(2), Cents::new(-50));
    let debts = vec![jizifin_auditor::settlement::Debt { from: MemberId(2), to: MemberId(1), amount: Cents::new(50) }];
    assert!(debts_settle(&net, &debts));
    let bad_debts = vec![jizifin_auditor::settlement::Debt { from: MemberId(2), to: MemberId(1), amount: Cents::new(49) }];
    assert!(!debts_settle(&net, &bad_debts));
}

#[test]
fn offset_rule_across_disjoint_categories_merges_components() {
    let mut snap = Snapshot::default();
    (1..=4).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);

    // Couple A (M1, M2) shares Cat 1; M1 pays 10,000
    snap.overrides.insert(EntryId(1), vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(2), pct: 50.0 }]);
    add_entry(&mut snap, 1, (1, 1), 10000);

    // Couple B (M3, M4) shares Cat 2; M3 pays 10,000
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(3), pct: 50.0 }, Share { member: MemberId(4), pct: 50.0 }]);
    add_entry(&mut snap, 2, (2, 3), 10000);

    // Offset rule: debit M1 (Cat 1) -> credit M3 (Cat 2)
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(3), credit_category: CategoryId(2), is_active: true,
    });

    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();

    // The offset rule ties M1 and M3 together, so all four settle in one component.
    assert_eq!(s.components.len(), 1);
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn circular_three_way_debt_simplification() {
    let mut snap = Snapshot::default();
    (1..=3).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);

    // M1 pays 30.00 for M2
    snap.overrides.insert(EntryId(1), vec![Share { member: MemberId(2), pct: 100.0 }]);
    add_entry(&mut snap, 1, (1, 1), 3000);

    // M2 pays 30.00 for M3
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(3), pct: 100.0 }]);
    add_entry(&mut snap, 2, (1, 2), 3000);

    // M3 pays 30.00 for M1
    snap.overrides.insert(EntryId(3), vec![Share { member: MemberId(1), pct: 100.0 }]);
    add_entry(&mut snap, 3, (1, 3), 3000);

    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();

    // Circular debt resolves completely to zero net transfers
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::ZERO));
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::ZERO));
    assert_eq!(s.net.get(&MemberId(3)).copied(), Some(Cents::ZERO));
    assert!(s.debts.is_empty());
    assert_eq!(s.transfer_total, Cents::ZERO);
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

fn setup_joint_exclusion_snap() -> Snapshot {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    snap.accounts.push(jizifin_domain::entities::Account {
        id: jizifin_domain::ids::AccountId(1), name: ct("Joint"), safety_margin_pct: 0.0,
        expected_override: None, deposit_mode: jizifin_domain::entities::DepositMode::Equal,
        members: Vec::new(), categories: vec![jizifin_domain::entities::AccountCategory {
            category: CategoryId(2), expected: Cents::new(10000),
        }],
    });
    snap
}

fn add_joint_entries(snap: &mut Snapshot) {
    let mut e1 = Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 15), amount: Cents::new(5000),
        member: Some(MemberId(1)), category: Some(CategoryId(1)), account: Some(jizifin_domain::ids::AccountId(1)),
        project: None, tag: None, recurring: None, name: Some(ct("J")), note: None,
    };
    snap.entries.push(e1.clone());
    e1.id = EntryId(2); e1.category = Some(CategoryId(2)); e1.account = None;
    snap.entries.push(e1);
    add_entry(snap, 3, (1, 1), 10000);
}

#[test]
fn joint_account_charges_excluded_but_out_of_pocket_joint_category_settles() {
    let mut snap = setup_joint_exclusion_snap();
    add_joint_entries(&mut snap);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    // Entry 1 is charged to the joint account and stays out; entry 2 (joint category, paid personally) settles.
    let categories: Vec<CategoryId> = s.rows.iter().map(|r| r.bucket.category).collect();
    assert_eq!(categories, vec![CategoryId(1), CategoryId(2)]);
    assert_eq!(s.rows.iter().map(|r| r.total).sum::<Cents>(), Cents::new(15000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn settlement_scope_member_filtering() {
    let mut snap = Snapshot::default();
    (1..=3).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 3), 10000);
    add_entry(&mut snap, 2, (1, 3), 10000);
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(3), pct: 50.0 }]);
    let mut scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    scope.members = Some(std::collections::BTreeSet::from([MemberId(1), MemberId(2)]));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.rows.len(), 1);
    assert_eq!(s.rows[0].total, Cents::new(10000));
}

#[test]
fn settlement_explain_and_entry_shares() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let calc = SettlementCalculator::new(&snap, &scope);
    let lines = calc.explain();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("via equal"));
    assert!(calc.entry_shares(EntryId(1)).is_some());
    assert!(calc.entry_shares(EntryId(999)).is_none());
}

#[test]
fn settlement_refund_reverses_debt_flow() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), -10000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].from, MemberId(1));
    assert_eq!(s.debts[0].to, MemberId(2));
    assert_eq!(s.debts[0].amount, Cents::new(5000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn offset_rules_inactive_or_zero_paid_ignored() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(2), credit_category: CategoryId(2), is_active: false,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::new(5000)));
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::new(-5000)));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn multi_party_mesh_debt_simplification() {
    let mut snap = Snapshot::default();
    (1..=4).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 12000);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::new(9000)));
    assert_eq!(s.debts.len(), 3);
    assert_eq!(s.transfer_total, Cents::new(9000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn settlement_ignores_non_expense_entries() {
    let mut snap = Snapshot::default();
    add_member(&mut snap, 1);
    add_member(&mut snap, 2);
    add_category(&mut snap, 1, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    let mut income_entry = snap.entries[0].clone();
    income_entry.id = EntryId(2);
    income_entry.kind = EntryKind::Income;
    snap.entries.push(income_entry);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.rows.len(), 1);
    assert_eq!(s.debts[0].amount, Cents::new(5000));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

fn setup_large_household_mesh() -> Snapshot {
    let mut snap = Snapshot::default();
    (1..=20).for_each(|id| add_member(&mut snap, id));
    (1..=5).for_each(|id| add_category(&mut snap, id, PayMode::Shared));
    for i in 1..=20 {
        let cat = ((i - 1) % 5) + 1;
        add_entry(&mut snap, i, (cat, i), i * 1000);
    }
    snap
}

#[test]
fn large_household_mesh_settlement_performance() {
    let snap = setup_large_household_mesh();
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let start = std::time::Instant::now();
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 200, "Settlement took too long: {elapsed:?}");
    assert!(!s.debts.is_empty());
    assert!(s.debts.len() < 20, "Debts were not simplified: {}", s.debts.len());
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}


#[test]
fn inactive_offset_rule_with_both_sides_paying_is_ignored() {
    let mut snap = Snapshot::default();
    (1..=4).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    snap.overrides.insert(EntryId(1), vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(2), pct: 50.0 }]);
    add_entry(&mut snap, 1, (1, 1), 10000);
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(3), pct: 50.0 }, Share { member: MemberId(4), pct: 50.0 }]);
    add_entry(&mut snap, 2, (2, 3), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(3), credit_category: CategoryId(2), is_active: false,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.components.len(), 2);
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn offset_rules_chained_across_three_parties() {
    let mut snap = Snapshot::default();
    (1..=3).for_each(|id| add_member(&mut snap, id));
    (1..=3).for_each(|id| add_category(&mut snap, id, PayMode::Shared));
    add_entry(&mut snap, 1, (1, 1), 12000);
    add_entry(&mut snap, 2, (2, 2), 6000);
    add_entry(&mut snap, 3, (3, 3), 3000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(2), credit_category: CategoryId(2), is_active: true,
    });
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(2), debit_member: MemberId(2), debit_category: CategoryId(2),
        credit_member: MemberId(3), credit_category: CategoryId(3), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn offset_rules_self_debit_credit_noop() {
    let mut snap = Snapshot::default();
    (1..=2).for_each(|id| add_member(&mut snap, id));
    (1..=2).for_each(|id| add_category(&mut snap, id, PayMode::Shared));
    add_entry(&mut snap, 1, (1, 1), 10000);
    add_entry(&mut snap, 2, (2, 1), 5000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(1), credit_category: CategoryId(2), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::new(7500)));
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::new(-7500)));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn offset_rules_negative_refund_amount_produces_zero_offset() {
    let mut snap = Snapshot::default();
    (1..=2).for_each(|id| add_member(&mut snap, id));
    (1..=2).for_each(|id| add_category(&mut snap, id, PayMode::Shared));
    add_entry(&mut snap, 1, (1, 1), -4000);
    add_entry(&mut snap, 2, (2, 2), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(2), credit_category: CategoryId(2), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.net.get(&MemberId(1)).copied(), Some(Cents::new(-7000)));
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::new(7000)));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}

#[test]
fn active_offset_rule_with_zero_paid_does_not_merge_components() {
    let mut snap = Snapshot::default();
    (1..=4).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Shared);
    add_category(&mut snap, 2, PayMode::Shared);
    add_category(&mut snap, 3, PayMode::Shared);
    add_category(&mut snap, 4, PayMode::Shared);
    snap.overrides.insert(EntryId(1), vec![Share { member: MemberId(1), pct: 50.0 }, Share { member: MemberId(2), pct: 50.0 }]);
    add_entry(&mut snap, 1, (1, 1), 10000);
    snap.overrides.insert(EntryId(2), vec![Share { member: MemberId(3), pct: 50.0 }, Share { member: MemberId(4), pct: 50.0 }]);
    add_entry(&mut snap, 2, (2, 3), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(3),
        credit_member: MemberId(3), credit_category: CategoryId(4), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    assert_eq!(s.components.len(), 2, "Components should remain isolated when offset is zero");
}


#[test]
fn offset_rule_counts_spend_in_personal_pay_categories() {
    let mut snap = Snapshot::default();
    (1..=2).for_each(|id| add_member(&mut snap, id));
    add_category(&mut snap, 1, PayMode::Personal);
    add_category(&mut snap, 2, PayMode::Shared);
    add_entry(&mut snap, 1, (1, 1), 10000);
    add_entry(&mut snap, 2, (2, 2), 10000);
    snap.offset_rules.push(OffsetRule {
        id: OffsetRuleId(1), debit_member: MemberId(1), debit_category: CategoryId(1),
        credit_member: MemberId(2), credit_category: CategoryId(2), is_active: true,
    });
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let s = SettlementCalculator::new(&snap, &scope).calculate();
    // Shared split leaves M2 +5,000 and M1 -5,000; the offset moves another 10,000 from M1 to M2.
    assert_eq!(s.net.get(&MemberId(2)).copied(), Some(Cents::new(15000)));
    assert!(settlement_invariants(&scope).verify(&s, &snap).is_ok());
}
