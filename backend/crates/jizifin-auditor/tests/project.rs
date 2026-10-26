use chrono::NaiveDate;
use jizifin_auditor::project::{ProjectSettlement, ProjectSettlementCalculator, project_invariants};
use jizifin_auditor::Snapshot;
use jizifin_domain::entities::{Category, CategoryKind, Entry, EntryKind, PayMode, Project};
use jizifin_domain::ids::{CategoryId, EntryId, MemberId, ProjectId};
use jizifin_domain::{Cents, Ciphertext};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn setup_project() -> (Snapshot, Project) {
    let mut snap = Snapshot::default();
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Materials"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    let project = Project {
        id: ProjectId(1), name: ct("Solar_Panels"), color: "#10b981".into(),
        target: Cents::new(1000000), target_date: Some(d(2027, 1, 1)),
        members: vec![MemberId(1), MemberId(2)],
    };
    snap.projects.push(project.clone());
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 1),
        amount: Cents::new(600000), member: Some(MemberId(1)), category: Some(CategoryId(1)),
        account: None, project: Some(ProjectId(1)), tag: None, recurring: None,
        name: Some(ct("Panels")), note: None,
    });
    (snap, project)
}

fn assert_participants(settlement: &ProjectSettlement) {
    let alice = settlement.participants.iter().find(|p| p.member == MemberId(1)).unwrap();
    assert_eq!(alice.funding, Cents::new(600000));
    assert_eq!(alice.liability, Cents::new(300000));
    assert_eq!(alice.net, Cents::new(300000));

    let bob = settlement.participants.iter().find(|p| p.member == MemberId(2)).unwrap();
    assert_eq!(bob.funding, Cents::ZERO);
    assert_eq!(bob.liability, Cents::new(300000));
    assert_eq!(bob.net, Cents::new(-300000));
}

#[test]
fn project_settlement_direct_funding_and_debts() {
    let (snap, project) = setup_project();
    let s = ProjectSettlementCalculator::new(&snap, &project).calculate();
    assert_eq!(s.total_spent, Cents::new(600000));
    assert_eq!(s.participants.len(), 2);
    assert_participants(&s);

    assert_eq!(s.debts.len(), 1);
    assert_eq!(s.debts[0].from, MemberId(2));
    assert_eq!(s.debts[0].to, MemberId(1));
    assert_eq!(s.debts[0].amount, Cents::new(300000));
    assert!(project_invariants().verify(&s, &snap).is_ok());
}

fn test_joint_account() -> jizifin_domain::entities::Account {
    jizifin_domain::entities::Account {
        id: jizifin_domain::ids::AccountId(1), name: ct("Joint"), safety_margin_pct: 0.0,
        expected_override: None, deposit_mode: jizifin_domain::entities::DepositMode::Equal,
        members: vec![
            jizifin_domain::entities::AccountMember { member: MemberId(1), fixed_deposit: None, due_day: 1 },
            jizifin_domain::entities::AccountMember { member: MemberId(2), fixed_deposit: None, due_day: 1 },
        ],
        categories: Vec::new(),
    }
}

fn setup_payerless_joint_project() -> (Snapshot, Project) {
    let mut snap = Snapshot::default();
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Materials"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    snap.members.push(jizifin_domain::entities::Member { id: MemberId(1), name: ct("M1"), color: "#000".into(), is_active: true });
    snap.members.push(jizifin_domain::entities::Member { id: MemberId(2), name: ct("M2"), color: "#000".into(), is_active: true });
    let project = Project {
        id: ProjectId(1), name: ct("Garden"), color: "#10b981".into(),
        target: Cents::new(100000), target_date: None, members: vec![MemberId(1), MemberId(2)],
    };
    snap.projects.push(project.clone());
    snap.accounts.push(test_joint_account());
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 1),
        amount: Cents::new(40000), member: None, category: Some(CategoryId(1)),
        account: Some(jizifin_domain::ids::AccountId(1)), project: Some(ProjectId(1)),
        tag: None, recurring: None, name: Some(ct("Soil")), note: None,
    });
    (snap, project)
}

#[test]
fn project_joint_expense_without_member_is_shared_equally() {
    let (snap, project) = setup_payerless_joint_project();
    let s = ProjectSettlementCalculator::new(&snap, &project).calculate();
    assert!(project_invariants().verify(&s, &snap).is_ok());
    assert!(s.participants.iter().all(|p| p.liability == Cents::new(20000)));
}

fn add_project_entries(snap: &mut Snapshot) {
    snap.entries.push(Entry {
        id: EntryId(1), kind: EntryKind::Expense, date: d(2026, 7, 1),
        amount: Cents::new(50000), member: Some(MemberId(1)), category: Some(CategoryId(1)),
        account: None, project: Some(ProjectId(1)), tag: None, recurring: None,
        name: Some(ct("Wood")), note: None,
    });
    snap.entries.push(Entry {
        id: EntryId(2), kind: EntryKind::Expense, date: d(2026, 7, 2),
        amount: Cents::new(30000), member: Some(MemberId(2)), category: Some(CategoryId(1)),
        account: None, project: Some(ProjectId(1)), tag: None, recurring: None,
        name: Some(ct("Paint")), note: None,
    });
}

fn setup_three_member_project() -> (Snapshot, Project) {
    let mut snap = Snapshot::default();
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Materials"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    (1..=3).for_each(|id| {
        snap.members.push(jizifin_domain::entities::Member { id: MemberId(id), name: ct(&format!("M{id}")), color: "#000".into(), is_active: true });
    });
    let project = Project {
        id: ProjectId(1), name: ct("Renovation"), color: "#10b981".into(),
        target: Cents::new(120000), target_date: None, members: vec![MemberId(1), MemberId(2), MemberId(3)],
    };
    snap.projects.push(project.clone());
    add_project_entries(&mut snap);
    (snap, project)
}

#[test]
fn project_three_member_mixed_joint_and_direct_funding() {
    let (snap, project) = setup_three_member_project();
    let s = ProjectSettlementCalculator::new(&snap, &project).calculate();
    assert_eq!(s.total_spent, Cents::new(80000));
    assert_eq!(s.participants.len(), 3);
    let total_liab: Cents = s.participants.iter().map(|p| p.liability).sum();
    assert_eq!(total_liab, Cents::new(80000));
    assert!(project_invariants().verify(&s, &snap).is_ok());
}

#[test]
fn joint_equity_uses_deposits_from_earlier_months() {
    let (mut snap, project) = setup_payerless_joint_project();
    snap.entries.push(Entry {
        id: EntryId(2), kind: EntryKind::Deposit, date: d(2026, 6, 10), amount: Cents::new(50000),
        member: Some(MemberId(1)), category: None, account: Some(jizifin_domain::ids::AccountId(1)),
        project: None, tag: None, recurring: None, name: None, note: None,
    });
    let s = ProjectSettlementCalculator::new(&snap, &project).calculate();
    let funding = |m: i64| s.participants.iter().find(|p| p.member == MemberId(m)).unwrap().funding;
    // No July deposits, yet M1's June savings fund the whole expense.
    assert_eq!(funding(1), Cents::new(40000));
    assert_eq!(funding(2), Cents::ZERO);
    assert!(project_invariants().verify(&s, &snap).is_ok());
}
