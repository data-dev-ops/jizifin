//! Auditor failures.

use jizifin_domain::ids::{AccountId, EntryId, ProjectId};
use thiserror::Error;

/// Why the Auditor refused to return a value.
#[derive(Debug, Error)]
pub enum AuditError {
    /// The snapshot could not be loaded.
    #[error("snapshot load failed: {0}")]
    Load(String),
    /// A computed value violated an invariant; it is never returned.
    #[error("audit invariant {0} failed")]
    InvariantFailed(&'static str),
    /// The requested joint account does not exist.
    #[error("account {0} not found")]
    UnknownAccount(AccountId),
    /// The entry is not a payable expense in the scoped month.
    #[error("expense {0} not found")]
    UnknownEntry(EntryId),
    /// The requested project does not exist.
    #[error("project {0} not found")]
    UnknownProject(ProjectId),
}
