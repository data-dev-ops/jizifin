use chrono::NaiveDate;
use jizifin_auditor::joint::{DepositStatus, JointBalanceCalculator, joint_invariants};
use jizifin_auditor::{AuditScope, Snapshot};
use jizifin_domain::entities::{Account, AccountCategory, AccountMember, DepositMode, Entry, EntryKind};
use jizifin_domain::ids::{AccountId, CategoryId, EntryId, MemberId};
use jizifin_domain::{Cents, Ciphertext, Month};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

fn ct(name: &str) -> Ciphertext {
    Ciphertext::parse(name).unwrap()
}

fn setup_joint_account() -> Account {
    Account {
        id: AccountId(1), name: ct("Household_Pool"), safety_margin_pct: 10.0,
        expected_override: None, deposit_mode: DepositMode::Equal,
        members: vec![
            AccountMember { member: MemberId(1), fixed_deposit: None, due_day: 5 },
            AccountMember { member: MemberId(2), fixed_deposit: None, due_day: 20 },
        ],
        categories: vec![
            AccountCategory { category: CategoryId(1), expected: Cents::new(50000) },
            AccountCategory { category: CategoryId(2), expected: Cents::new(50000) },
        ],
    }
}

fn joint_entry(id: i64, kind: EntryKind, amount: i64, day: u32) -> Entry {
    Entry {
        id: EntryId(id), kind, date: d(2026, 7, day), amount: Cents::new(amount),
        member: (kind != EntryKind::Correction).then_some(MemberId(1)),
        category: (kind == EntryKind::Expense).then_some(CategoryId(1)),
        account: Some(AccountId(1)), project: None, tag: None, recurring: None,
        name: Some(ct("Item")), note: None,
    }
}

#[test]
fn balance_calculation_and_corrections() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 100000, 1));
    snap.entries.push(joint_entry(2, EntryKind::Expense, 40000, 5));
    snap.entries.push(joint_entry(3, EntryKind::Correction, -5000, 6));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.balance, Cents::new(55000));
    assert_eq!(dash.actual_total, Cents::new(40000));
    assert_eq!(dash.month_surplus, Cents::new(60000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}

#[test]
fn status_pending_and_below_target() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    let month = Month::parse("2026-07").unwrap();

    snap.entries.push(joint_entry(1, EntryKind::Deposit, 50000, 2));
    let scope_mid = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 10));
    let dash_mid = JointBalanceCalculator::new(&snap, &scope_mid, &account).calculate();
    assert_eq!(dash_mid.status, DepositStatus::Pending { due_day: 20 });

    let scope_end = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash_end = JointBalanceCalculator::new(&snap, &scope_end, &account).calculate();
    assert_eq!(dash_end.status, DepositStatus::BelowTarget);
}

#[test]
fn status_target_met_when_threshold_reached() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    let month = Month::parse("2026-07").unwrap();

    snap.entries.push(joint_entry(1, EntryKind::Deposit, 50000, 2));
    let mut e2 = joint_entry(2, EntryKind::Deposit, 70000, 20);
    e2.member = Some(MemberId(2));
    snap.entries.push(e2);

    let scope_end = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash_met = JointBalanceCalculator::new(&snap, &scope_end, &account).calculate();
    assert_eq!(dash_met.status, DepositStatus::TargetMet);
}

#[test]
fn joint_balance_excludes_future_dated_entries() {
    // Balance is the month-end balance: entries dated after the scoped month are ignored.
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());

    // July deposit of 1000.00
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 100000, 1));

    // August expense of 300.00 (in the future relative to July)
    let mut aug_expense = joint_entry(2, EntryKind::Expense, 30000, 15);
    aug_expense.date = d(2026, 8, 15);
    snap.entries.push(aug_expense);

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();

    assert_eq!(dash.balance, Cents::new(100000));
}

#[test]
fn overdrawn_joint_account_negative_balance_and_top_up() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());

    // Deposit 500.00, but expense 800.00 -> balance -300.00
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 50000, 1));
    snap.entries.push(joint_entry(2, EntryKind::Expense, 80000, 5));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();

    assert_eq!(dash.balance, Cents::new(-30000));
    assert_eq!(dash.target, Cents::new(110000));
    assert_eq!(dash.top_up, Cents::new(140000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}

#[test]
fn joint_status_all_members_paid_below_target_is_below_target() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    let month = Month::parse("2026-07").unwrap();
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 10000, 2));
    let mut e2 = joint_entry(2, EntryKind::Deposit, 10000, 8);
    e2.member = Some(MemberId(2));
    snap.entries.push(e2);

    let scope_mid = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 10));
    let dash = JointBalanceCalculator::new(&snap, &scope_mid, &account).calculate();
    assert_eq!(dash.status, DepositStatus::BelowTarget);
}

#[test]
fn joint_status_past_and_future_months() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    let month = Month::parse("2026-07").unwrap();

    let past_scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 8, 10));
    let dash_past = JointBalanceCalculator::new(&snap, &past_scope, &account).calculate();
    assert_eq!(dash_past.status, DepositStatus::BelowTarget);

    let fut_scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 6, 10));
    let dash_fut = JointBalanceCalculator::new(&snap, &fut_scope, &account).calculate();
    assert_eq!(dash_fut.status, DepositStatus::BelowTarget);
}

#[test]
fn joint_due_day_clamping_for_short_months() {
    let mut snap = Snapshot::default();
    let mut account = setup_joint_account();
    account.members[0].due_day = 31;
    snap.accounts.push(account.clone());

    let month = Month::parse("2026-06").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 6, 30));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.members[0].due_day, 30);
}

#[test]
fn joint_category_usage_zero_expected_and_unspent() {
    let mut snap = Snapshot::default();
    let mut account = setup_joint_account();
    account.categories[0].expected = Cents::ZERO;
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Expense, 25000, 10));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.categories[0].pct_used, 0.0);
    assert_eq!(dash.categories[1].actual, Cents::ZERO);
    assert_eq!(dash.categories[1].pct_used, 0.0);
}

#[test]
fn joint_account_expected_override_beats_categories() {
    let mut snap = Snapshot::default();
    let mut account = setup_joint_account();
    account.expected_override = Some(Cents::new(150000));
    snap.accounts.push(account.clone());

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.expected_total, Cents::new(150000));
    assert_eq!(dash.target, Cents::new(165000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}

#[test]
fn joint_income_entry_contributes_to_balance() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Income, 25000, 5));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.balance, Cents::new(25000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}

#[test]
fn joint_balance_with_surplus_zeroes_top_up() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 200000, 5));

    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.balance, Cents::new(200000));
    assert_eq!(dash.top_up, Cents::ZERO);
    assert_eq!(dash.month_surplus, Cents::new(200000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}

#[test]
fn joint_due_day_exact_today_is_below_target() {
    let mut snap = Snapshot::default();
    let mut account = setup_joint_account();
    account.members[0].due_day = 15;
    account.members[1].due_day = 10;
    snap.accounts.push(account.clone());
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 15));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.status, DepositStatus::BelowTarget);
}

#[test]
fn joint_corrections_positive_and_negative_in_deposited_total() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Deposit, 50000, 2));
    snap.entries.push(joint_entry(2, EntryKind::Correction, 10000, 3));
    snap.entries.push(joint_entry(3, EntryKind::Correction, -5000, 4));
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.deposited_total, Cents::new(60000));
    assert_eq!(dash.balance, Cents::new(55000));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}


#[test]
fn unbudgeted_refund_and_joint_income_keep_dashboard_consistent() {
    let mut snap = Snapshot::default();
    let account = setup_joint_account();
    snap.accounts.push(account.clone());
    snap.entries.push(joint_entry(1, EntryKind::Expense, 20000, 3));
    let mut refund = joint_entry(2, EntryKind::Expense, -30000, 4);
    refund.category = Some(CategoryId(9));
    snap.entries.push(refund);
    let mut interest = joint_entry(3, EntryKind::Income, 1500, 5);
    interest.category = None;
    snap.entries.push(interest);
    let month = Month::parse("2026-07").unwrap();
    let scope = AuditScope::account_to_date(month, AccountId(1), d(2026, 7, 31));
    let dash = JointBalanceCalculator::new(&snap, &scope, &account).calculate();
    assert_eq!(dash.actual_total, Cents::new(-10000));
    assert_eq!(dash.other_actual, Cents::new(-30000));
    assert_eq!(dash.deposited_total, Cents::new(1500));
    assert_eq!(dash.month_surplus, Cents::new(11500));
    assert!(joint_invariants().verify(&dash, &snap).is_ok());
}
