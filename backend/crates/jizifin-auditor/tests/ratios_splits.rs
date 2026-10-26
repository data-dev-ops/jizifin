use chrono::NaiveDate;
use jizifin_auditor::ratios::{SalaryRatioCalculator, ratio_invariants};
use jizifin_auditor::splits::{SplitOverviewCalculator, SplitProposer, SplitStrategy, overview_invariants, proposal_invariants};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::entities::{Category, CategoryKind, Frequency, Job, Member, PayMode};
use jizifin_domain::ids::{CategoryId, JobId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn members_snap() -> Snapshot {
    let mut snap = Snapshot::default();
    snap.members.push(Member { id: MemberId(1), name: ct("Alice"), color: "#111".into(), is_active: true });
    snap.members.push(Member { id: MemberId(2), name: ct("Bob"), color: "#222".into(), is_active: true });
    snap
}

#[test]
fn salary_ratios_proportional() {
    let mut snap = members_snap();
    snap.jobs.push(Job {
        id: JobId(1), member: MemberId(1), amount: Cents::new(300000), frequency: Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to: None, name: ct("W1"), notes: None,
    });
    snap.jobs.push(Job {
        id: JobId(2), member: MemberId(2), amount: Cents::new(100000), frequency: Frequency::Monthly,
        valid_from: d(2026, 1, 1), valid_to: None, name: ct("W2"), notes: None,
    });

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let ratios = SalaryRatioCalculator::new(&snap, &scope).calculate();
    assert_eq!(ratios.pct.get(&MemberId(1)).copied(), Some(75));
    assert_eq!(ratios.pct.get(&MemberId(2)).copied(), Some(25));
    assert!(ratio_invariants().verify(&ratios, &snap).is_ok());
}

#[test]
fn salary_ratios_equal_fallback() {
    let snap = members_snap();
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let ratios = SalaryRatioCalculator::new(&snap, &scope).calculate();
    assert_eq!(ratios.pct.get(&MemberId(1)).copied(), Some(50));
    assert_eq!(ratios.pct.get(&MemberId(2)).copied(), Some(50));
    assert!(ratio_invariants().verify(&ratios, &snap).is_ok());
}

#[test]
fn split_overview_and_proposer() {
    let mut snap = members_snap();
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("SharedCat"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let overview = SplitOverviewCalculator::new(&snap, &scope).calculate();
    assert_eq!(overview.categories.len(), 1);
    assert!(overview_invariants().verify(&overview, &snap).is_ok());

    let proposal = SplitProposer::new(&snap, &scope).propose(SplitStrategy::Even);
    assert_eq!(proposal.pct.len(), 2);
    assert!(proposal_invariants().verify(&proposal, &snap).is_ok());
}
