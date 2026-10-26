//! Reviewed rows to entry drafts: validation and bundling of grouped debits.

use crate::CsvError;
use serde::{Deserialize, Serialize};

/// A reviewed statement row, as edited in the UI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportItem {
    /// Selected for import.
    pub include: bool,
    /// Money received.
    pub is_credit: bool,
    /// Absolute cents.
    pub amount_cents: i64,
    /// ISO date.
    pub date: String,
    /// Counterparty, becomes the entry name.
    pub beneficiary: String,
    /// Payer or earner id.
    pub member: Option<i64>,
    /// Category id.
    pub category: Option<i64>,
    /// Tag id (expenses only).
    pub tag: Option<i64>,
    /// Fold into the bundle entry.
    #[serde(default)]
    pub group_to_other: bool,
}

/// An entry ready for `POST /entries/batch` once `label` is encrypted into `name`.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct EntryDraft {
    /// `expense` or `income`.
    pub kind: &'static str,
    /// ISO date.
    pub date: String,
    /// Cents.
    pub amount: i64,
    /// Member id.
    pub member: Option<i64>,
    /// Category id.
    pub category: i64,
    /// Tag id.
    pub tag: Option<i64>,
    /// Plaintext name, encrypted by the caller.
    pub label: String,
}

/// Import preparation with the bundle category for grouped debits.
pub struct ImportPlan {
    bundle_category: Option<i64>,
}

impl ImportPlan {
    /// Plan using `bundle_category` for grouped rows.
    pub fn new(bundle_category: Option<i64>) -> Self {
        Self { bundle_category }
    }

    /// Drafts for every included non-zero row; grouped debits become one entry.
    pub fn prepare(&self, items: &[ImportItem]) -> Result<Vec<EntryDraft>, CsvError> {
        let active: Vec<&ImportItem> = items.iter().filter(|i| i.include && i.amount_cents > 0).collect();
        let (grouped, single): (Vec<&ImportItem>, Vec<&ImportItem>) = active.into_iter().partition(|i| !i.is_credit && i.group_to_other);
        let mut drafts = single.into_iter().map(draft).collect::<Result<Vec<_>, _>>()?;
        drafts.extend(self.bundle(&grouped)?);
        Ok(drafts)
    }

    fn bundle(&self, grouped: &[&ImportItem]) -> Result<Option<EntryDraft>, CsvError> {
        let Some(first) = grouped.first() else { return Ok(None) };
        let category = self.bundle_category.ok_or_else(|| CsvError("Grouped expenses need an OTHER category.".into()))?;
        Ok(Some(EntryDraft {
            kind: "expense",
            date: grouped.iter().map(|i| i.date.clone()).max().unwrap_or_default(),
            amount: grouped.iter().map(|i| i.amount_cents).sum(),
            member: first.member,
            category,
            tag: None,
            label: format!("Other expenses ({} items bundled)", grouped.len()),
        }))
    }
}

fn draft(item: &ImportItem) -> Result<EntryDraft, CsvError> {
    let kind = if item.is_credit { "income" } else { "expense" };
    let category = item.category.ok_or_else(|| CsvError(format!("Category is required for {kind} \"{}\" ({}).", item.beneficiary, item.date)))?;
    Ok(EntryDraft {
        kind,
        date: item.date.clone(),
        amount: item.amount_cents,
        member: item.member,
        category,
        tag: if item.is_credit { None } else { item.tag },
        label: item.beneficiary.clone(),
    })
}

/// Counts and totals of the rows selected for import.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    /// Included debit rows.
    pub expenses: usize,
    /// Included credit rows.
    pub incomes: usize,
    /// Included debits folded into the bundle.
    pub grouped: usize,
    /// Σ included debits.
    pub expense_cents: i64,
    /// Σ included credits.
    pub income_cents: i64,
    /// Σ grouped debits.
    pub grouped_cents: i64,
    /// Included rows still missing a category (grouped rows need none).
    pub uncategorized: usize,
}

impl ImportSummary {
    /// Summary over `items`.
    pub fn of(items: &[ImportItem]) -> Self {
        items.iter().filter(|i| i.include && i.amount_cents > 0).fold(Self::default(), |mut s, i| {
            s.add(i);
            s
        })
    }

    fn add(&mut self, item: &ImportItem) {
        let grouped = !item.is_credit && item.group_to_other;
        if item.is_credit {
            (self.incomes, self.income_cents) = (self.incomes + 1, self.income_cents + item.amount_cents);
        } else {
            (self.expenses, self.expense_cents) = (self.expenses + 1, self.expense_cents + item.amount_cents);
        }
        if grouped {
            (self.grouped, self.grouped_cents) = (self.grouped + 1, self.grouped_cents + item.amount_cents);
        }
        self.uncategorized += usize::from(!grouped && item.category.is_none());
    }
}

/// Indices of rows sharing row `index`'s beneficiary and direction (the category cascade).
pub fn cascade_targets(items: &[ImportItem], index: usize) -> Vec<usize> {
    let Some(source) = items.get(index) else { return Vec::new() };
    let same = |i: &ImportItem| i.beneficiary == source.beneficiary && i.is_credit == source.is_credit;
    items.iter().enumerate().filter(|(_, i)| same(i)).map(|(n, _)| n).collect()
}

/// Indices of included debits below `max_cents` (candidates for the bundle).
pub fn micro_debits(items: &[ImportItem], max_cents: i64) -> Vec<usize> {
    let micro = |i: &ImportItem| i.include && !i.is_credit && i.amount_cents < max_cents;
    items.iter().enumerate().filter(|(_, i)| micro(i)).map(|(n, _)| n).collect()
}
