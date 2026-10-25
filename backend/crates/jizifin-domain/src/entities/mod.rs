//! Domain entities mirroring the DuckDB schema.

mod account;
mod category;
mod entry;
mod income;
mod member;
mod project;
mod recurring;
mod rules;

pub use account::{Account, AccountCategory, AccountMember, DepositMode};
pub use category::{Agreement, Category, CategoryKind, PayMode, Share};
pub use entry::{Entry, EntryKind, ShareOverride};
pub use income::{Frequency, IncomeOverride, Job};
pub use member::Member;
pub use project::{Budget, BudgetSource, Project, Tag};
pub use recurring::{Recurrence, RecurringTemplate};
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

/// Stored label of an enum value, the inverse of [`parse_variant`].
pub(crate) fn label_of<T: Copy + PartialEq>(value: T, table: &[(&'static str, T)]) -> &'static str {
    table.iter().find(|(_, v)| *v == value).map_or("", |(label, _)| label)
}
