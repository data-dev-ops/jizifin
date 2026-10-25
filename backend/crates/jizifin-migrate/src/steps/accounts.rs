//! Joint accounts (roster, deposit plans, categories) and projects.

use super::{Steps, ct, opt_date};
use crate::error::MigrateError;
use crate::legacy::LegacyAccount;
use jizifin_domain::Cents;
use jizifin_domain::entities::{AccountCategory, AccountMember, DepositMode};
use jizifin_domain::ids::MemberId;
use jizifin_store::inputs::{AccountInput, ProjectInput};
use jizifin_store::repo::{Accounts, Projects};
use std::collections::BTreeMap;

impl Steps<'_> {
    pub(super) fn accounts(&mut self) -> Result<(), MigrateError> {
        let accounts = self.legacy.accounts()?;
        for account in &accounts {
            let input = self.account_input(account)?;
            self.ids.accounts.insert(account.id, Accounts::create(self.conn, &input)?);
        }
        self.report.count("joint accounts", accounts.len());
        Ok(())
    }

    fn account_input(&self, account: &LegacyAccount) -> Result<AccountInput, MigrateError> {
        Ok(AccountInput {
            name: ct(&account.name)?,
            safety_margin_pct: account.margin,
            expected_override: account.expected_total.map(Cents::new),
            deposit_mode: Self::deposit_mode(&account.mode)?,
            members: self.account_members(account.id)?,
            categories: self.account_categories(account.id)?,
        })
    }

    fn deposit_mode(raw: &str) -> Result<DepositMode, MigrateError> {
        match raw {
            "even" => Ok(DepositMode::Equal),
            "salary" => Ok(DepositMode::SalaryRatio),
            "manual" => Ok(DepositMode::Fixed),
            other => Err(MigrateError::Data(format!("unknown deposit_split_mode '{other}'"))),
        }
    }

    /// Roster ∪ deposit plans; every member gets a fixed amount (0 when unplanned) and a due day.
    fn account_members(&self, account: i64) -> Result<Vec<AccountMember>, MigrateError> {
        let mut members: BTreeMap<MemberId, AccountMember> = BTreeMap::new();
        for (_, user) in self.legacy.account_members()?.into_iter().filter(|(a, _)| *a == account) {
            let member = self.ids.member(&user)?;
            members.insert(member, AccountMember { member, fixed_deposit: Some(Cents::ZERO), due_day: 1 });
        }
        for plan in self.legacy.deposit_plans()?.into_iter().filter(|p| p.account_id == account) {
            let member = self.ids.member(&plan.user)?;
            members.insert(member, AccountMember { member, fixed_deposit: Some(Cents::new(plan.amount)), due_day: plan.day });
        }
        Ok(members.into_values().collect())
    }

    fn account_categories(&self, account: i64) -> Result<Vec<AccountCategory>, MigrateError> {
        let rows = self.legacy.account_categories()?.into_iter().filter(|c| c.account_id == account);
        let mut by_category = BTreeMap::new();
        for row in rows {
            let category = self.ids.expense_category(&row.category)?;
            let slot = by_category.entry(category).or_insert(AccountCategory { category, expected: Cents::ZERO });
            slot.expected = slot.expected.max(Cents::new(row.expected));
        }
        Ok(by_category.into_values().collect())
    }

    pub(super) fn projects(&mut self) -> Result<(), MigrateError> {
        let projects = self.legacy.projects()?;
        let users = self.legacy.project_users()?;
        for p in &projects {
            let members = users.iter().filter(|(id, _)| *id == p.id).map(|(_, u)| self.ids.member(u)).collect::<Result<Vec<_>, _>>()?;
            let input = ProjectInput {
                name: ct(&p.name)?, color: "#64748b".into(), target: Cents::new(p.target),
                target_date: opt_date(p.target_date.as_deref())?, members,
            };
            self.ids.projects.insert(p.id, Projects::create(self.conn, &input)?);
        }
        self.report.count("projects", projects.len());
        Ok(())
    }
}
