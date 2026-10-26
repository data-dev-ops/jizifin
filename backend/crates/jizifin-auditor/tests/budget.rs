use chrono::NaiveDate;
use jizifin_auditor::budget::{BudgetEvaluator, BudgetProposer, proposal_invariants, status_invariants};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::entities::{Budget, BudgetSource, Category, CategoryKind, Entry, EntryKind, PayMode};
use jizifin_domain::ids::{AccountId, BudgetId, CategoryId, EntryId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};
use std::collections::BTreeSet;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn add_cat(snap: &mut Snapshot, id: i64) {
    let cat = Category {
        id: CategoryId(id), kind: CategoryKind::Expense,
        name: ct(&format!("Cat{id}")), pay_mode: PayMode::Shared,
    };
    snap.categories.insert(cat.id, cat);
}

fn standing_budget(id: i64, cat: i64, from: &str, limit: i64) -> Budget {
    Budget {
        id: BudgetId(id), category: CategoryId(cat), valid_from: Month::parse(from).unwrap(),
        valid_to: None, limit: Cents::new(limit), source: BudgetSource::Manual,
    }
}

fn month_budget(id: i64, cat: i64, month: &str, limit: i64) -> Budget {
    let m = Month::parse(month).unwrap();
    Budget {
        id: BudgetId(id), category: CategoryId(cat), valid_from: m,
        valid_to: Some(m), limit: Cents::new(limit), source: BudgetSource::Manual,
    }
}

fn range_budget(id: i64, cat: i64, (from, to): (&str, &str), limit: i64) -> Budget {
    Budget {
        id: BudgetId(id), category: CategoryId(cat),
        valid_from: Month::parse(from).unwrap(),
        valid_to: Some(Month::parse(to).unwrap()),
        limit: Cents::new(limit), source: BudgetSource::Manual,
    }
}

fn setup_evaluator_snap() -> Snapshot {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 50000));
    snap.entries.push(test_expense(1, 1, 20000, d(2026, 7, 10)));
    snap
}

#[test]
fn budget_status_evaluation_and_health_pct() {
    let snap = setup_evaluator_snap();
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows.len(), 1);
    let row = &report.rows[0];
    assert_eq!(row.limit, Cents::new(50000));
    assert_eq!(row.spent, Cents::new(20000));
    assert_eq!(row.remaining, Cents::new(30000));
    assert_eq!(row.pct_used, 40.0);
    assert_eq!(report.summary.health_pct, 100.0);
    assert!(status_invariants().verify(&report, &snap).is_ok());
}

#[test]
fn month_specific_budget_beats_standing_budget() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 50000));
    snap.budgets.push(month_budget(2, 1, "2026-07", 60000));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].limit, Cents::new(60000));
}

#[test]
fn budget_proposals_from_trailing_lookback() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.entries.push(test_expense(1, 1, 10000, d(2026, 4, 15)));
    snap.entries.push(test_expense(2, 1, 15000, d(2026, 5, 15)));
    snap.entries.push(test_expense(3, 1, 20000, d(2026, 6, 15)));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::trailing(month, 3, d(2026, 7, 31));
    let proposals = BudgetProposer::new(&snap, &scope).propose();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].proposed, Cents::new(15000));
    assert!(proposal_invariants().verify(&proposals, &snap).is_ok());
}

#[test]
fn budget_evaluator_overspent_category_and_health() {
    let mut snap = Snapshot::default();
    let cat = Category { id: CategoryId(1), kind: CategoryKind::Expense, name: ct("Dining"), pay_mode: PayMode::Shared };
    snap.categories.insert(cat.id, cat);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 50000));
    // Spent 750.00 against 500.00 limit -> 250.00 over budget
    snap.entries.push(test_expense(1, 1, 75000, d(2026, 7, 20)));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::month(month, d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();

    assert_eq!(report.rows.len(), 1);
    let row = &report.rows[0];
    assert_eq!(row.limit, Cents::new(50000));
    assert_eq!(row.spent, Cents::new(75000));
    assert_eq!(row.remaining, Cents::new(-25000));
    assert_eq!(row.pct_used, 150.0);
    assert_eq!(report.summary.health_pct, 0.0);
    assert!(status_invariants().verify(&report, &snap).is_ok());
}

#[test]
fn budget_summary_empty_and_zero_limits() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 0));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].pct_used, 0.0);
    assert_eq!(report.summary.configured, 0);
    assert_eq!(report.summary.safe, 0);
    assert_eq!(report.summary.health_pct, 100.0);
    assert_eq!(report.summary.usage_pct, None);
    assert!(status_invariants().verify(&report, &snap).is_ok());
}

#[test]
fn budget_summary_safe_threshold_boundary() {
    let mut snap = Snapshot::default();
    (1..=3).for_each(|id| add_cat(&mut snap, id));
    (1..=3).for_each(|id| snap.budgets.push(standing_budget(id, id, "2026-01", 100000)));
    snap.entries.push(test_expense(1, 1, 89900, d(2026, 7, 1)));
    snap.entries.push(test_expense(2, 2, 90000, d(2026, 7, 1)));
    snap.entries.push(test_expense(3, 3, 90100, d(2026, 7, 1)));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.summary.configured, 3);
    assert_eq!(report.summary.safe, 1);
    assert_eq!(report.rows[0].pct_used, 89.9);
    assert_eq!(report.rows[1].pct_used, 90.0);
    assert_eq!(report.rows[2].pct_used, 90.1);
    assert!(status_invariants().verify(&report, &snap).is_ok());
}

#[test]
fn budget_evaluator_scope_member_and_account_filtering() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 50000));
    let mut e1 = test_expense(1, 1, 10000, d(2026, 7, 5));
    e1.account = Some(AccountId(10));
    let mut e2 = test_expense(2, 1, 5000, d(2026, 7, 6));
    e2.member = Some(MemberId(2));
    e2.account = Some(AccountId(10));
    let e3 = test_expense(3, 1, 8000, d(2026, 7, 7));
    snap.entries.extend([e1, e2, e3]);
    let mut scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    scope.members = Some(BTreeSet::from([MemberId(1)]));
    scope.account = Some(AccountId(10));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].spent, Cents::new(10000));
}

#[test]
fn budget_precedence_standing_start_and_id_tiebreak() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(10, 1, "2026-01", 40000));
    snap.budgets.push(standing_budget(11, 1, "2026-04", 50000));
    snap.budgets.push(standing_budget(12, 1, "2026-04", 60000));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].limit, Cents::new(60000));
}

#[test]
fn budget_precedence_bounded_start_and_id_tiebreak() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(range_budget(1, 1, ("2026-01", "2026-12"), 30000));
    snap.budgets.push(range_budget(2, 1, ("2026-05", "2026-09"), 45000));
    snap.budgets.push(range_budget(3, 1, ("2026-05", "2026-09"), 55000));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].limit, Cents::new(55000));
}

#[test]
fn budget_refund_reduces_spent_and_increases_remaining() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-01", 50000));
    snap.entries.push(test_expense(1, 1, 35000, d(2026, 7, 10)));
    snap.entries.push(test_expense(2, 1, -10000, d(2026, 7, 15)));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].spent, Cents::new(25000));
    assert_eq!(report.rows[0].remaining, Cents::new(25000));
    assert_eq!(report.rows[0].pct_used, 50.0);
    assert!(status_invariants().verify(&report, &snap).is_ok());
}

#[test]
fn budget_proposals_empty_history_and_negative_spend() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.entries.push(test_expense(1, 1, 20000, d(2026, 7, 10)));
    snap.entries.push(test_expense(2, 1, 20000, d(2026, 2, 10)));
    let scope = AuditScope::trailing(Month::parse("2026-07").unwrap(), 3, d(2026, 7, 31));
    assert!(BudgetProposer::new(&snap, &scope).propose().is_empty());
    snap.entries.push(test_expense(3, 1, -5000, d(2026, 5, 10)));
    assert!(BudgetProposer::new(&snap, &scope).propose().is_empty());
}

#[test]
fn budget_proposals_rounding_whole_units() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.entries.push(test_expense(1, 1, 101, d(2026, 4, 15)));
    snap.entries.push(test_expense(2, 1, 100, d(2026, 5, 15)));
    snap.entries.push(test_expense(3, 1, 100, d(2026, 6, 15)));
    let scope = AuditScope::trailing(Month::parse("2026-07").unwrap(), 3, d(2026, 7, 31));
    let proposals = BudgetProposer::new(&snap, &scope).propose();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].proposed, Cents::new(200));
    assert!(proposal_invariants().verify(&proposals, &snap).is_ok());
}

#[test]
fn budget_precedence_bounded_beats_later_standing() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.budgets.push(standing_budget(1, 1, "2026-05", 50000));
    snap.budgets.push(range_budget(2, 1, ("2026-01", "2026-12"), 40000));
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].limit, Cents::new(40000));
}

#[test]
fn budget_evaluator_ignores_non_expense_entries() {
    let mut snap = setup_evaluator_snap();
    let mut non_exp = test_expense(2, 1, 25000, d(2026, 7, 12));
    non_exp.kind = EntryKind::Income;
    snap.entries.push(non_exp.clone());
    non_exp.id = EntryId(3);
    non_exp.kind = EntryKind::Deposit;
    snap.entries.push(non_exp.clone());
    non_exp.id = EntryId(4);
    non_exp.kind = EntryKind::Correction;
    snap.entries.push(non_exp);
    let scope = AuditScope::month(Month::parse("2026-07").unwrap(), d(2026, 7, 31));
    let report = BudgetEvaluator::new(&snap, &scope).evaluate();
    assert_eq!(report.rows[0].spent, Cents::new(20000));
}

#[test]
fn budget_proposals_single_active_month_in_lookback() {
    let mut snap = Snapshot::default();
    add_cat(&mut snap, 1);
    snap.entries.push(test_expense(1, 1, 30000, d(2026, 5, 10)));
    let scope = AuditScope::trailing(Month::parse("2026-07").unwrap(), 3, d(2026, 7, 31));
    let proposals = BudgetProposer::new(&snap, &scope).propose();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].proposed, Cents::new(10000));
    assert_eq!(proposals[0].history, vec![Cents::ZERO, Cents::new(30000), Cents::ZERO]);
    assert!(proposal_invariants().verify(&proposals, &snap).is_ok());
}

fn test_expense(id: i64, cat: i64, amount: i64, date: NaiveDate) -> Entry {
    Entry {
        id: EntryId(id), kind: EntryKind::Expense, date, amount: Cents::new(amount),
        member: Some(MemberId(1)), category: Some(CategoryId(cat)),
        account: None, project: None, tag: None, recurring: None,
        name: Some(ct("Exp")), note: None,
    }
}

