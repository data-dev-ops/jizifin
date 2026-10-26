use chrono::NaiveDate;
use jizifin_auditor::cashflow::{CashFlowCalculator, cash_flow_invariants};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::entities::{Category, CategoryKind, Entry, EntryKind, Frequency, Job, Member, PayMode};
use jizifin_domain::ids::{CategoryId, EntryId, JobId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn sample_snapshot() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Living"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(300000),
        frequency: Frequency::Monthly, valid_from: d(2026, 1, 1), valid_to: None,
        name: ct("Salary"), notes: None,
    });
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 15),
        amount: Cents::new(120000), member: Some(MemberId(1)), category: Some(CategoryId(1)),
        account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Rent")), note: None,
    });
    snap
}

#[test]
fn cash_flow_income_spending_and_net() {
    let snap = sample_snapshot();
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let flow = CashFlowCalculator::new(&snap, &scope).calculate();

    assert_eq!(flow.income, Cents::new(300000));
    assert_eq!(flow.spent, Cents::new(120000));
    assert_eq!(flow.net, Cents::new(180000));
    assert_eq!(flow.savings_rate_pct, Some(60.0));
    assert!(cash_flow_invariants().verify(&flow, &snap).is_ok());
}

fn setup_deficit_snap() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Living"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(100000),
        frequency: Frequency::Monthly, valid_from: d(2026, 1, 1), valid_to: None,
        name: ct("Salary"), notes: None,
    });
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 15),
        amount: Cents::new(250000), member: Some(MemberId(1)), category: Some(CategoryId(1)),
        account: None, project: None, tag: None, recurring: None,
        name: Some(ct("EmergencyRepair")), note: None,
    });
    snap
}

#[test]
fn cash_flow_severe_deficit_negative_savings_rate() {
    let snap = setup_deficit_snap();
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let flow = CashFlowCalculator::new(&snap, &scope).calculate();

    assert_eq!(flow.income, Cents::new(100000));
    assert_eq!(flow.spent, Cents::new(250000));
    assert_eq!(flow.net, Cents::new(-150000));
    assert_eq!(flow.savings_rate_pct, Some(-150.0));
    assert!(cash_flow_invariants().verify(&flow, &snap).is_ok());
}

#[test]
fn joint_deposits_count_as_outlay_and_joint_charges_do_not() {
    let mut snap = sample_snapshot();
    let entry = |id, kind, amount, account| Entry {
        id: EntryId(id), kind, date: d(2026, 7, 10), amount: Cents::new(amount),
        member: Some(MemberId(1)), category: (kind == EntryKind::Expense).then_some(CategoryId(1)),
        account, project: None, tag: None, recurring: None, name: Some(ct("Joint")), note: None,
    };
    let joint = Some(jizifin_domain::ids::AccountId(1));
    snap.entries.push(entry(2, EntryKind::Deposit, 50000, joint));
    snap.entries.push(entry(3, EntryKind::Expense, 150000, joint));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let flow = CashFlowCalculator::new(&snap, &scope).calculate();
    // Own rent 1,200.00 + deposit 500.00; the 1,500.00 joint charge is funded by deposits already counted.
    assert_eq!(flow.spent, Cents::new(170000));
    assert!(cash_flow_invariants().verify(&flow, &snap).is_ok());
}
