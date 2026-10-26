use jizifin_auditor::deposit::{DepositProposer, deposit_invariants};
use jizifin_auditor::Snapshot;
use jizifin_domain::entities::{Account, AccountCategory, AccountMember, DepositMode, Frequency, Job, Member};
use jizifin_domain::ids::{AccountId, CategoryId, JobId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};
use chrono::NaiveDate;

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn test_account(mode: DepositMode, margin: f64, exp1: i64, exp2: i64) -> Account {
    Account {
        id: AccountId(1), name: ct("Joint"), safety_margin_pct: margin,
        expected_override: None, deposit_mode: mode,
        members: vec![
            AccountMember { member: MemberId(1), fixed_deposit: Some(Cents::new(30000)), due_day: 1 },
            AccountMember { member: MemberId(2), fixed_deposit: Some(Cents::new(20000)), due_day: 5 },
        ],
        categories: vec![
            AccountCategory { category: CategoryId(1), expected: Cents::new(exp1) },
            AccountCategory { category: CategoryId(2), expected: Cents::new(exp2) },
        ],
    }
}

#[test]
fn equal_deposit_mode_with_safety_margin() {
    let account = test_account(DepositMode::Equal, 10.0, 50000, 50000);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.expected_total, Cents::new(100000));
    assert_eq!(proposal.target, Cents::new(110000));
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(55000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::new(55000)));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

fn snap_with_jobs() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.members.push(Member { id: MemberId(2), name: ct("Bob"), color: "#222".into(), is_active: true });
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(300000), frequency: Frequency::Monthly,
        valid_from: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), valid_to: None, name: ct("Work"), notes: None,
    });
    snap.jobs.push(Job {
        id: JobId(2), member: MemberId(2), amount: Cents::new(100000), frequency: Frequency::Monthly,
        valid_from: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), valid_to: None, name: ct("Work2"), notes: None,
    });
    snap
}

#[test]
fn salary_ratio_deposit_mode() {
    let account = test_account(DepositMode::SalaryRatio, 0.0, 60000, 40000);
    let snap = snap_with_jobs();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.target, Cents::new(100000));
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(75000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::new(25000)));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

#[test]
fn zero_income_deposit_fallback() {
    let account = test_account(DepositMode::SalaryRatio, 0.0, 60000, 40000);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(50000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::new(50000)));
}

#[test]
fn rounding_up_to_step() {
    let account = test_account(DepositMode::Equal, 0.0, 33333, 0);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let step = Cents::new(500);
    let proposal = DepositProposer::new(&snap, &account, month).rounded_to(Some(step)).propose();
    assert!(proposal.rounded.is_some());
    let rounded = proposal.rounded.as_ref().unwrap();
    assert_eq!(rounded.get(&MemberId(1)).copied(), Some(Cents::new(17000)));
    assert_eq!(rounded.get(&MemberId(2)).copied(), Some(Cents::new(17000)));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

#[test]
fn fixed_deposit_mode_uses_member_amounts() {
    let mut account = test_account(DepositMode::Fixed, 0.0, 50000, 50000);
    account.members[1].fixed_deposit = None;
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(30000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::ZERO));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

#[test]
fn salary_ratio_ignores_incomes_of_non_account_members() {
    let account = test_account(DepositMode::SalaryRatio, 0.0, 60000, 40000);
    let mut snap = snap_with_jobs();
    snap.jobs.push(Job {
        id: JobId(3), member: MemberId(3), amount: Cents::new(500000), frequency: Frequency::Monthly,
        valid_from: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), valid_to: None, name: ct("Work3"), notes: None,
    });
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(75000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::new(25000)));
    assert_eq!(proposal.per_member.get(&MemberId(3)), None);
}

#[test]
fn deposit_negative_safety_margin() {
    let account = test_account(DepositMode::Equal, -10.0, 50000, 50000);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.target, Cents::new(90000));
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(45000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::new(45000)));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

#[test]
fn deposit_invalid_step_ignored() {
    let account = test_account(DepositMode::Equal, 0.0, 50000, 50000);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let p_zero = DepositProposer::new(&snap, &account, month).rounded_to(Some(Cents::ZERO)).propose();
    assert!(p_zero.rounded.is_none());
    let p_neg = DepositProposer::new(&snap, &account, month).rounded_to(Some(Cents::new(-500))).propose();
    assert!(p_neg.rounded.is_none());
}

#[test]
fn deposit_zero_expected_total() {
    let account = test_account(DepositMode::Equal, 0.0, 0, 0);
    let snap = Snapshot::default();
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.target, Cents::ZERO);
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::ZERO));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

#[test]
fn salary_ratio_one_earning_one_zero_income() {
    let account = test_account(DepositMode::SalaryRatio, 0.0, 50000, 50000);
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.members.push(Member { id: MemberId(2), name: ct("Bob"), color: "#222".into(), is_active: true });
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(300000), frequency: Frequency::Monthly,
        valid_from: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), valid_to: None, name: ct("Work"), notes: None,
    });
    let month = Month::parse("2026-07").unwrap();
    let proposal = DepositProposer::new(&snap, &account, month).propose();
    assert_eq!(proposal.per_member.get(&MemberId(1)).copied(), Some(Cents::new(100000)));
    assert_eq!(proposal.per_member.get(&MemberId(2)).copied(), Some(Cents::ZERO));
    assert!(deposit_invariants().verify(&proposal, &snap).is_ok());
}

