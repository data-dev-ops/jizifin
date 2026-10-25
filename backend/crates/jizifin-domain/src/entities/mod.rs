//! Domain entities mirroring the DuckDB schema.

mod account;
mod category;
mod entry;
mod income;
mod member;
mod project;
mod rules;

pub use account::{Account, AccountCategory, AccountMember, DepositMode};
pub use category::{Agreement, Category, CategoryKind, PayMode, Share};
pub use entry::{Entry, EntryKind, ShareOverride};
pub use income::{Frequency, IncomeOverride, Job};
pub use member::Member;
pub use project::{Budget, BudgetSource, Project, Tag};
pub use rules::{MonthSeal, OffsetRule};

/// Parses a lowercase enum label, used by every text-backed enum.
pub(crate) fn parse_variant<T: Copy>(
    kind: &'static str,
    raw: &str,
    table: &[(&str, T)],
) -> Result<T, crate::DomainError> {
    table
        .iter()
        .find(|(label, _)| *label == raw)
        .map(|(_, v)| *v)
        .ok_or_else(|| crate::DomainError::UnknownVariant { kind, value: raw.to_owned() })
}
