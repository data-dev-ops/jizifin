use jizifin_wasm::beneficiary::ing_beneficiary;
use jizifin_wasm::import::{ImportItem, ImportPlan, ImportSummary, cascade_targets, micro_debits};
use jizifin_wasm::statement::{Bank, Statement};

const SAMPLE_CSV: &str = include_str!("fixtures/ing_statement_sample.csv");

#[test]
fn parse_ing_sample_statement_stats() {
    let stmt = Statement::parse(SAMPLE_CSV, Bank::Ing).unwrap();
    assert_eq!(stmt.stats.total, 44);
    assert_eq!(stmt.stats.credits_count, 5);
    assert_eq!(stmt.stats.expenses_count, 39);
    assert_eq!(stmt.transactions.len(), 44);
}

#[test]
fn parse_ing_sample_statement_first_row() {
    let stmt = Statement::parse(SAMPLE_CSV, Bank::Ing).unwrap();
    let first = &stmt.transactions[0];
    assert_eq!(first.row_index, 1);
    assert_eq!(first.amount_cents, 390);
    assert!(!first.is_credit);
    assert_eq!(first.date, "2026-08-01");
}

#[test]
fn unsupported_and_empty_csv() {
    assert!(Statement::parse("", Bank::Ing).is_err());
    assert!(Bank::parse("KBC").is_err());
    assert!(Bank::parse("UNKNOWN").is_err());
}

#[test]
fn beneficiary_extraction() {
    let desc = "Betaling Debit Mastercard 01/08/26 - 16.09 uur - DECATHLON 0252 88000 - EPINAL - FRA Kaartnummer 5229";
    assert_eq!(ing_beneficiary(desc), "DECATHLON");

    let transfer = "Overschrijving in euro (SEPA): ING app Naar: Proximus NV - BE31435411161155 Mededeling: ***801***";
    assert_eq!(ing_beneficiary(transfer), "Proximus NV");
}

#[test]
fn cascade_and_micro_debits() {
    let items = vec![
        make_debit("Shop A", 500, None),
        make_debit("Shop B", 200, None),
        make_debit("Shop A", 800, None),
    ];
    let targets = cascade_targets(&items, 0);
    assert_eq!(targets, vec![0, 2]);

    let micros = micro_debits(&items, 600);
    assert_eq!(micros, vec![0, 1]);
}

#[test]
fn import_summary_and_plan_bundling() {
    let items = vec![
        make_debit("Bakery", 250, Some(10)),
        make_debit("Coffee", 300, Some(10)),
        make_credit("Salary", 300000, Some(20)),
    ];
    let summary = ImportSummary::of(&items);
    assert_eq!(summary.expenses, 2);
    assert_eq!(summary.incomes, 1);
    assert_eq!(summary.expense_cents, 550);
    assert_eq!(summary.income_cents, 300000);

    let plan = ImportPlan::new(Some(99));
    let drafts = plan.prepare(&items).unwrap();
    assert_eq!(drafts.len(), 3);
}

fn make_debit(beneficiary: &str, cents: i64, cat: Option<i64>) -> ImportItem {
    item(beneficiary, cents, false, cat)
}

fn make_credit(beneficiary: &str, cents: i64, cat: Option<i64>) -> ImportItem {
    item(beneficiary, cents, true, cat)
}

fn item(beneficiary: &str, cents: i64, is_credit: bool, cat: Option<i64>) -> ImportItem {
    ImportItem {
        include: true,
        is_credit,
        amount_cents: cents,
        date: "2026-08-01".into(),
        beneficiary: beneficiary.into(),
        member: Some(1),
        category: cat,
        tag: None,
        group_to_other: false,
    }
}
