//! The Auditor: sole owner of contentious calculations.
//!
//! Every request loads an immutable [`Snapshot`] from the store, computes in memory,
//! verifies the result against its invariants and returns an [`AuditedResult`].

pub mod allocation;
pub mod auditor;
pub mod budget;
pub mod cache;
pub mod deposit;
pub mod error;
pub mod income;
pub mod invariant;
pub mod joint;
pub mod ratios;
pub mod result;
pub mod scope;
pub mod settlement;
pub mod shares;
pub mod snapshot;

pub use auditor::Auditor;
pub use cache::DataVersion;
pub use error::AuditError;
pub use result::AuditedResult;
pub use scope::AuditScope;
pub use snapshot::{Snapshot, SnapshotLoader};
