//! Legacy natural key → new surrogate key maps.

use crate::error::MigrateError;
use jizifin_domain::ids::{AccountId, CategoryId, MemberId, ProjectId, TagId};
use std::collections::HashMap;

/// Every key mapping built while migrating.
#[derive(Default)]
pub struct IdMap {
    /// Member by legacy name ciphertext.
    pub members: HashMap<String, MemberId>,
    /// Expense category by legacy name ciphertext.
    pub expense_categories: HashMap<String, CategoryId>,
    /// Income category by legacy name ciphertext.
    pub income_categories: HashMap<String, CategoryId>,
    /// Account by legacy id.
    pub accounts: HashMap<i64, AccountId>,
    /// Project by legacy id.
    pub projects: HashMap<i64, ProjectId>,
    /// Tag by legacy id.
    pub tags: HashMap<i64, TagId>,
}

impl IdMap {
    /// Member for a legacy name.
    pub fn member(&self, name: &str) -> Result<MemberId, MigrateError> {
        self.members.get(name).copied().ok_or_else(|| MigrateError::Data(format!("unknown member '{name}'")))
    }

    /// Expense category for a legacy name.
    pub fn expense_category(&self, name: &str) -> Result<CategoryId, MigrateError> {
        let found = self.expense_categories.get(name).copied();
        found.ok_or_else(|| MigrateError::Data(format!("unknown expense category '{name}'")))
    }

    /// Account for a legacy id; `None` falls back to the first account.
    pub fn account(&self, legacy: Option<i64>) -> Result<AccountId, MigrateError> {
        let id = legacy.or_else(|| self.accounts.keys().min().copied());
        id.and_then(|i| self.accounts.get(&i).copied()).ok_or_else(|| MigrateError::Data("joint entry without any account".into()))
    }

    /// Optional project for a legacy id.
    pub fn project(&self, legacy: Option<i64>) -> Option<ProjectId> {
        legacy.and_then(|i| self.projects.get(&i).copied())
    }

    /// Optional tag for a legacy id.
    pub fn tag(&self, legacy: Option<i64>) -> Option<TagId> {
        legacy.and_then(|i| self.tags.get(&i).copied())
    }
}
