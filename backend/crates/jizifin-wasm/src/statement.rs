//! Statement parsing: header detection and typed transaction rows.

use crate::amount::EuroAmount;
use crate::beneficiary::ing_beneficiary;
use crate::date::iso_date;
use crate::tokenizer::Tokenizer;
use crate::CsvError;
use serde::Serialize;

/// Supported bank export formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bank {
    /// ING Belgium.
    Ing,
}

impl Bank {
    /// Parses a bank code.
    pub fn parse(code: &str) -> Result<Self, CsvError> {
        match code {
            "ING" => Ok(Self::Ing),
            "KBC" => Err(CsvError("KBC Bank export format is not yet supported.".into())),
            other => Err(CsvError(format!("Unsupported bank selection: {other}"))),
        }
    }
}

/// One statement row, with review defaults the UI edits in place.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// 1-based data row number.
    pub row_index: usize,
    /// Description as exported.
    pub raw_description: String,
    /// ISO booking date.
    pub date: String,
    /// Date as exported.
    pub display_date: String,
    /// Absolute cents.
    pub amount_cents: i64,
    /// Money received.
    pub is_credit: bool,
    /// Extracted counterparty.
    pub beneficiary: String,
    /// Selected for import.
    pub include: bool,
    /// Folded into one bundle entry.
    pub group_to_other: bool,
}

/// Counts and totals shown above the review table.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    /// Row count.
    pub total: usize,
    /// Debit rows.
    pub expenses_count: usize,
    /// Credit rows.
    pub credits_count: usize,
    /// Debit cents.
    pub total_expense_cents: i64,
    /// Credit cents.
    pub total_credit_cents: i64,
}

impl Stats {
    fn add(&mut self, tx: &Transaction) {
        self.total += 1;
        if tx.is_credit {
            self.credits_count += 1;
            self.total_credit_cents += tx.amount_cents;
        } else {
            self.expenses_count += 1;
            self.total_expense_cents += tx.amount_cents;
        }
    }
}

/// Parsed statement.
#[derive(Debug, Serialize)]
pub struct Statement {
    /// Rows in file order.
    pub transactions: Vec<Transaction>,
    /// Summary.
    pub stats: Stats,
}

impl Statement {
    /// Parses `text` exported by `bank`.
    pub fn parse(text: &str, bank: Bank) -> Result<Self, CsvError> {
        let Bank::Ing = bank;
        let rows = Tokenizer::new(',').rows(text);
        let (header, data) = rows.split_first().filter(|(_, d)| !d.is_empty()).ok_or_else(|| CsvError("CSV file is empty or missing data rows.".into()))?;
        let columns = Columns::detect(header)?;
        Ok(Self::collect(data.iter().enumerate().filter_map(|(i, row)| columns.transaction(i + 1, row))))
    }

    fn collect(rows: impl Iterator<Item = Transaction>) -> Self {
        let transactions: Vec<Transaction> = rows.collect();
        let mut stats = Stats::default();
        transactions.iter().for_each(|tx| stats.add(tx));
        Self { transactions, stats }
    }
}

/// Column positions resolved from the header (English or Dutch).
struct Columns {
    date: usize,
    amount: usize,
    description: usize,
}

impl Columns {
    fn detect(header: &[String]) -> Result<Self, CsvError> {
        let names: Vec<String> = header.iter().map(|h| h.trim().to_lowercase()).collect();
        let find = |keys: &[&str]| names.iter().position(|h| keys.iter().any(|k| h.contains(k)));
        let amount = find(&["amount", "bedrag"]).ok_or_else(|| CsvError("Could not find Amount column in CSV.".into()))?;
        let date = find(&["booking date", "boekingsdatum"]).or_else(|| find(&["value date", "valutadatum"])).unwrap_or(4);
        let description = find(&["description", "omschrijving"]).unwrap_or(8);
        Ok(Self { date, amount, description })
    }

    fn transaction(&self, row_index: usize, row: &[String]) -> Option<Transaction> {
        let cell = |i: usize| row.get(i).map_or("", String::as_str);
        let raw_amount = row.get(self.amount)?;
        let amount = EuroAmount::parse(raw_amount).filter(|_| !raw_amount.is_empty())?;
        Some(Transaction {
            row_index,
            raw_description: cell(self.description).to_owned(),
            date: iso_date(cell(self.date)),
            display_date: cell(self.date).to_owned(),
            amount_cents: amount.magnitude(),
            is_credit: amount.is_credit(),
            beneficiary: ing_beneficiary(cell(self.description)),
            include: true,
            group_to_other: false,
        })
    }
}
