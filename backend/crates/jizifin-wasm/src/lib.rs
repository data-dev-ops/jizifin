//! Bank statement CSV parsing for the browser.
//!
//! Plaintext-only work: the browser reads a statement file, this crate turns
//! it into typed rows and, after the user has assigned categories, into
//! ledger entries ready for encryption and `POST /entries/batch`.

pub mod amount;
pub mod beneficiary;
pub mod date;
pub mod import;
pub mod statement;
pub mod tokenizer;

use import::{ImportItem, ImportPlan, ImportSummary, cascade_targets, micro_debits};
use statement::{Bank, Statement};
use wasm_bindgen::prelude::*;

/// Parses a statement export for `bank` (`"ING"`).
#[wasm_bindgen(js_name = parseStatement)]
pub fn parse_statement(text: &str, bank: &str) -> Result<JsValue, JsError> {
    let statement = Statement::parse(text, Bank::parse(bank)?)?;
    Ok(serde_wasm_bindgen::to_value(&statement)?)
}

/// Turns reviewed rows into entry drafts; `bundle_category` receives grouped rows.
#[wasm_bindgen(js_name = prepareImport)]
pub fn prepare_import(items: JsValue, bundle_category: Option<u32>) -> Result<JsValue, JsError> {
    let items: Vec<ImportItem> = serde_wasm_bindgen::from_value(items)?;
    let drafts = ImportPlan::new(bundle_category.map(i64::from)).prepare(&items)?;
    Ok(serde_wasm_bindgen::to_value(&drafts)?)
}

/// Counts and totals of the rows selected for import.
#[wasm_bindgen(js_name = summarizeImport)]
pub fn summarize_import(items: JsValue) -> Result<JsValue, JsError> {
    let items: Vec<ImportItem> = serde_wasm_bindgen::from_value(items)?;
    Ok(serde_wasm_bindgen::to_value(&ImportSummary::of(&items))?)
}

/// Rows that take row `index`'s category: same beneficiary, same direction.
#[wasm_bindgen(js_name = cascadeTargets)]
pub fn cascade(items: JsValue, index: usize) -> Result<Vec<usize>, JsError> {
    let items: Vec<ImportItem> = serde_wasm_bindgen::from_value(items)?;
    Ok(cascade_targets(&items, index))
}

/// Included debits below `max_cents`.
#[wasm_bindgen(js_name = microDebits)]
pub fn micro(items: JsValue, max_cents: u32) -> Result<Vec<usize>, JsError> {
    let items: Vec<ImportItem> = serde_wasm_bindgen::from_value(items)?;
    Ok(micro_debits(&items, i64::from(max_cents)))
}

/// Parse or import failure, surfaced to JS as an `Error` message.
#[derive(Debug, PartialEq, Eq)]
pub struct CsvError(pub String);

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CsvError {}
