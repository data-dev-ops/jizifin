//! Categories, split agreements, tags, budgets and the netting rule.

use super::{Steps, ct, date, month, opt_ct, opt_date};
use crate::error::MigrateError;
use crate::legacy::{LegacyAgreement, LegacyAllocation};
use jizifin_auditor::scope::LEDGER_START;
use jizifin_domain::entities::{BudgetSource, CategoryKind, PayMode, Share};
use jizifin_domain::{Cents, Month};
use jizifin_store::inputs::{AgreementInput, BudgetInput, CategoryInput, OffsetRuleInput, TagInput};
use jizifin_store::repo::{Agreements, Budgets, Categories, OffsetRules, Tags};
use std::collections::BTreeMap;

impl Steps<'_> {
    pub(super) fn categories(&mut self) -> Result<(), MigrateError> {
        let expense = self.legacy.expense_categories()?;
        for name in &expense {
            let id = self.create_category(CategoryKind::Expense, name, self.pay_mode(name))?;
            self.ids.expense_categories.insert(name.clone(), id);
        }
        let income: Vec<String> = self.legacy.income_categories()?.into_iter().filter(|c| *c != self.markers.salary).collect();
        for name in &income {
            let id = self.create_category(CategoryKind::Income, name, PayMode::Shared)?;
            self.ids.income_categories.insert(name.clone(), id);
        }
        self.report.count("expense categories", expense.len());
        self.report.count("income categories", income.len());
        Ok(())
    }

    fn pay_mode(&self, name: &str) -> PayMode {
        match self.markers.personal.iter().any(|p| p == name) {
            true => PayMode::Personal,
            false => PayMode::Shared,
        }
    }

    fn create_category(&self, kind: CategoryKind, name: &str, pay_mode: PayMode) -> Result<jizifin_domain::ids::CategoryId, MigrateError> {
        Ok(Categories::create(self.conn, &CategoryInput { kind, name: ct(name)?, pay_mode })?)
    }

    /// Active SCD2 agreements plus legacy baseline allocations as open-ended agreements from the ledger start.
    pub(super) fn agreements(&mut self) -> Result<(), MigrateError> {
        let allocations = self.legacy.allocations()?;
        let mut created = 0;
        for a in &self.legacy.agreements()? {
            created += usize::from(self.agreement(a, &allocations)?);
        }
        let baselines = self.baselines(&allocations)?;
        self.report.count("agreements", created + baselines);
        Ok(())
    }

    /// Legacy agreements without allocations never fixed a split (the equal fallback applied), so they are skipped.
    fn agreement(&mut self, a: &LegacyAgreement, allocations: &[LegacyAllocation]) -> Result<bool, MigrateError> {
        let shares = self.shares(allocations.iter().filter(|x| x.agreement_id == Some(a.id)))?;
        if shares.is_empty() {
            self.report.note(format!("skipped legacy agreement {} without allocations", a.id));
            return Ok(false);
        }
        let input = AgreementInput { valid_from: date(&a.start)?, valid_to: opt_date(a.end.as_deref())?, note: opt_ct(a.note.as_deref())?, shares };
        Agreements::create(self.conn, self.ids.expense_category(&a.category)?, &input)?;
        Ok(true)
    }

    fn baselines(&mut self, allocations: &[LegacyAllocation]) -> Result<usize, MigrateError> {
        let mut by_category: BTreeMap<&str, Vec<&LegacyAllocation>> = BTreeMap::new();
        allocations.iter().filter(|a| a.agreement_id.is_none()).for_each(|a| by_category.entry(&a.category).or_default().push(a));
        for (category, rows) in &by_category {
            let input = AgreementInput { valid_from: LEDGER_START, valid_to: None, note: None, shares: self.shares(rows.iter().copied())? };
            Agreements::create(self.conn, self.ids.expense_category(category)?, &input)?;
        }
        Ok(by_category.len())
    }

    fn shares<'r>(&self, rows: impl Iterator<Item = &'r LegacyAllocation>) -> Result<Vec<Share>, MigrateError> {
        rows.map(|r| Ok(Share { member: self.ids.member(&r.user)?, pct: r.pct })).collect()
    }

    pub(super) fn tags(&mut self) -> Result<(), MigrateError> {
        let tags = self.legacy.tags()?;
        for t in &tags {
            let input = TagInput {
                name: ct(&t.name)?, description: opt_ct(t.description.as_deref())?, color: t.color.clone(),
                valid_from: opt_date(t.start.as_deref())?, valid_to: opt_date(t.end.as_deref())?,
            };
            self.ids.tags.insert(t.id, Tags::create(self.conn, &input)?);
        }
        self.report.count("tags", tags.len());
        Ok(())
    }

    /// `month = 'ALL'` becomes a standing budget; a concrete month becomes a one-month override.
    pub(super) fn budgets(&mut self) -> Result<(), MigrateError> {
        let budgets = self.legacy.budgets()?;
        for b in &budgets {
            let (valid_from, valid_to) = match b.month.as_str() {
                "ALL" => (Month::of(LEDGER_START), None),
                raw => (month(raw)?, Some(month(raw)?)),
            };
            let input = BudgetInput { category: self.ids.expense_category(&b.category)?, valid_from, valid_to, limit: Cents::new(b.limit), source: BudgetSource::Manual };
            Budgets::create(self.conn, &input)?;
        }
        self.report.count("budgets", budgets.len());
        Ok(())
    }

    /// The hard-coded Jane/John "Combined Fixed" vs "Apartment" deduction becomes data.
    pub(super) fn offset_rule(&mut self) -> Result<(), MigrateError> {
        let m = self.markers;
        let found = (self.ids.members.get(&m.jane), self.ids.members.get(&m.john));
        let cats = (self.ids.expense_categories.get(&m.combined_fixed), self.ids.expense_categories.get(&m.apartment));
        let ((Some(jane), Some(john)), (Some(fixed), Some(apartment))) = (found, cats) else {
            self.report.note("offset rule skipped: Jane/John or their categories not found");
            return Ok(());
        };
        let input = OffsetRuleInput { credit_member: *jane, credit_category: *fixed, debit_member: *john, debit_category: *apartment, is_active: true };
        OffsetRules::create(self.conn, &input)?;
        self.report.count("offset rules", 1);
        Ok(())
    }
}
