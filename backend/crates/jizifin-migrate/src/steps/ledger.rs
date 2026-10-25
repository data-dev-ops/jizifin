//! Expenses, non-salary income, paid deposits, corrections, opening balances and recurring templates.

use super::{Steps, ct, date, month, opt_ct, opt_date};
use crate::error::MigrateError;
use crate::legacy::{LegacyExpense, LegacyRecurring};
use chrono::{NaiveDate, Utc};
use duckdb::params;
use jizifin_domain::entities::{EntryKind, Recurrence, Share};
use jizifin_domain::ids::{AccountId, RecurringId};
use jizifin_domain::{Cents, Ciphertext};
use jizifin_store::inputs::{EntryInput, RecurringInput};
use jizifin_store::repo::{Entries, Recurring};
use std::collections::BTreeMap;

/// Running joint-account balance per account: (signed sum, earliest date).
type Balances = BTreeMap<AccountId, (Cents, Option<NaiveDate>)>;

impl Steps<'_> {
    pub(super) fn ledger(&mut self) -> Result<(), MigrateError> {
        let mut balances = Balances::new();
        self.expenses(&mut balances)?;
        self.income(&mut balances)?;
        self.deposits(&mut balances)?;
        self.corrections(&mut balances)?;
        self.opening_balances(&balances)
    }

    fn entry(kind: EntryKind, date: NaiveDate, amount: i64) -> EntryInput {
        EntryInput {
            kind, date, amount: Cents::new(amount), member: None, category: None, account: None,
            project: None, tag: None, name: None, note: None, overrides: Vec::new(),
        }
    }

    fn track(balances: &mut Balances, input: &EntryInput) {
        let Some(account) = input.account else { return };
        let signed = if input.kind == EntryKind::Expense { -input.amount } else { input.amount };
        let slot = balances.entry(account).or_insert((Cents::ZERO, None));
        slot.0 += signed;
        slot.1 = Some(slot.1.map_or(input.date, |d| d.min(input.date)));
    }

    fn write(&self, balances: &mut Balances, input: &EntryInput) -> Result<(), MigrateError> {
        Entries::create(self.conn, input)?;
        Self::track(balances, input);
        Ok(())
    }

    fn expenses(&mut self, balances: &mut Balances) -> Result<(), MigrateError> {
        let expenses = self.legacy.expenses()?;
        let overrides = self.overrides()?;
        for e in &expenses {
            let input = self.expense_input(e, overrides.get(&e.id).cloned().unwrap_or_default())?;
            self.write(balances, &input)?;
        }
        self.report.count("expense entries", expenses.len());
        Ok(())
    }

    fn overrides(&self) -> Result<BTreeMap<i64, Vec<Share>>, MigrateError> {
        let mut map: BTreeMap<i64, Vec<Share>> = BTreeMap::new();
        for (expense, user, pct) in self.legacy.expense_overrides()? {
            map.entry(expense).or_default().push(Share { member: self.ids.member(&user)?, pct });
        }
        Ok(map)
    }

    fn expense_input(&self, e: &LegacyExpense, overrides: Vec<Share>) -> Result<EntryInput, MigrateError> {
        let account = if e.is_joint { Some(self.ids.account(e.account_id)?) } else { None };
        Ok(EntryInput {
            member: Some(self.ids.member(&e.payer)?), category: Some(self.ids.expense_category(&e.category)?), account,
            project: self.ids.project(e.project_id), tag: self.ids.tag(e.tag_id), name: Some(ct(&e.name)?), overrides,
            ..Self::entry(EntryKind::Expense, date(&e.date)?, e.cost)
        })
    }

    /// `SALARY` rows are skipped here; they become jobs.
    fn income(&mut self, balances: &mut Balances) -> Result<(), MigrateError> {
        let rows: Vec<_> = self.legacy.income()?.into_iter().filter(|i| i.category != self.markers.salary).collect();
        for i in &rows {
            let category = self.ids.income_categories.get(&i.category).copied();
            let account = if i.is_joint { Some(self.ids.account(None)?) } else { None };
            let input = EntryInput { member: Some(self.ids.member(&i.who)?), category, account, name: Some(ct(&i.name)?), ..Self::entry(EntryKind::Income, date(&i.date)?, i.amount) };
            self.write(balances, &input)?;
        }
        self.report.count("income entries", rows.len());
        Ok(())
    }

    fn deposits(&mut self, balances: &mut Balances) -> Result<(), MigrateError> {
        let deposits = self.legacy.paid_deposits()?;
        for d in &deposits {
            let day = opt_date(d.paid_date.as_deref())?.unwrap_or(month(&d.month)?.first_day());
            let input = EntryInput { member: Some(self.ids.member(&d.user)?), account: Some(self.ids.account(Some(d.account_id))?), ..Self::entry(EntryKind::Deposit, day, d.amount) };
            self.write(balances, &input)?;
        }
        self.report.count("deposit entries", deposits.len());
        Ok(())
    }

    fn corrections(&mut self, balances: &mut Balances) -> Result<(), MigrateError> {
        let corrections = self.legacy.corrections()?;
        for c in &corrections {
            let input = EntryInput { account: Some(self.ids.account(Some(c.account_id))?), note: opt_ct(c.note.as_deref())?, ..Self::entry(EntryKind::Correction, date(&c.date)?, c.amount) };
            self.write(balances, &input)?;
        }
        self.report.count("correction entries", corrections.len());
        Ok(())
    }

    /// Legacy accounts stored a balance; the new ledger derives it. A correction bridges the gap.
    fn opening_balances(&mut self, balances: &Balances) -> Result<(), MigrateError> {
        let mut bridged = 0;
        for account in self.legacy.accounts()? {
            let id = self.ids.account(Some(account.id))?;
            let (derived, first) = balances.get(&id).copied().unwrap_or((Cents::ZERO, None));
            let gap = Cents::new(account.balance) - derived;
            if !gap.is_zero() {
                let day = first.unwrap_or_else(|| Utc::now().date_naive());
                Entries::create(self.conn, &EntryInput { account: Some(id), ..Self::entry(EntryKind::Correction, day, gap.value()) })?;
                bridged += 1;
            }
        }
        self.report.count("opening balance corrections", bridged);
        Ok(())
    }

    pub(super) fn recurring(&mut self) -> Result<(), MigrateError> {
        let templates = self.legacy.recurring()?;
        let mut linked = 0;
        for t in &templates {
            let input = self.recurring_input(t)?;
            let id = Recurring::create(self.conn, &input)?;
            linked += self.link_generated(id, &input)?;
        }
        self.report.count("recurring templates", templates.len());
        self.report.count("entries linked to templates", linked);
        Ok(())
    }

    fn recurring_input(&self, t: &LegacyRecurring) -> Result<RecurringInput, MigrateError> {
        let valid_from = date(&t.start)?;
        let account = if t.is_joint { Some(self.ids.account(t.account_id)?) } else { None };
        Ok(RecurringInput {
            kind: EntryKind::Expense, amount: Cents::new(t.cost), member: self.ids.member(&t.payer)?,
            category: self.ids.expense_category(&t.category)?, account,
            day_of_month: t.day.unwrap_or_else(|| chrono::Datelike::day(&valid_from)),
            recurrence: Recurrence::parse(&t.frequency.replace("4-weekly", "four_weekly"))?,
            valid_from, valid_to: opt_date(t.end.as_deref())?, name: ct(&t.name)?, is_active: t.is_active,
        })
    }

    /// Links legacy entries the old scheduler generated, so the new one does not repeat them.
    fn link_generated(&self, id: RecurringId, t: &RecurringInput) -> Result<usize, MigrateError> {
        let sql = "UPDATE entries SET recurring_id = ? WHERE recurring_id IS NULL AND kind = 'expense' AND name_ct = ? \
                   AND member_id = ? AND category_id = ? AND amount_cents = ? AND entry_date >= ? AND entry_date <= COALESCE(?, entry_date)";
        let name: &Ciphertext = &t.name;
        let p = params![id.0, name.as_str(), t.member.0, t.category.0, t.amount.value(), t.valid_from, t.valid_to];
        Ok(self.conn.execute(sql, p).map_err(jizifin_store::StoreError::from)?)
    }
}
