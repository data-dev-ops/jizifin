//! Repositories: one stateless class per aggregate. Reads take a `Connection`,
//! writes take the `Transaction` opened by [`crate::Database::write`].

mod accounts;
mod agreements;
mod auth;
mod budgets;
mod categories;
mod console;
mod entries;
mod income;
mod members;
mod offsets;
mod projects;
mod recurring;
mod seals;
mod tags;

pub use accounts::Accounts;
pub use agreements::Agreements;
pub use auth::{Auth, Sessions};
pub use budgets::Budgets;
pub use categories::Categories;
pub use console::{Console, QueryResult};
pub use entries::{DuplicateMatch, Entries, EntryFilter, EntryPage, EntryView};
pub use income::{IncomeOverrides, Jobs};
pub use members::Members;
pub use offsets::OffsetRules;
pub use projects::Projects;
pub use recurring::Recurring;
pub use seals::Seals;
pub use tags::Tags;
