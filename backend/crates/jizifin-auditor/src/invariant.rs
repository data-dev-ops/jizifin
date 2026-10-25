//! Self-checks run on every result before it leaves the Auditor.

use crate::{AuditError, Snapshot};

/// A rule a computed value must satisfy.
pub trait Invariant<T>: Send + Sync {
    /// Stable identifier, reported on failure.
    fn id(&self) -> &'static str;
    /// True when `value` satisfies the rule.
    fn holds(&self, value: &T, snap: &Snapshot) -> bool;
}

/// Ordered list of invariants for one result type.
pub struct InvariantSet<T> {
    rules: Vec<Box<dyn Invariant<T>>>,
}

impl<T> InvariantSet<T> {
    /// Builds a set from boxed rules.
    pub fn new(rules: Vec<Box<dyn Invariant<T>>>) -> Self {
        Self { rules }
    }

    /// Runs every rule; returns the ids that held or the first failure.
    pub fn verify(&self, value: &T, snap: &Snapshot) -> Result<Vec<&'static str>, AuditError> {
        self.rules
            .iter()
            .map(|rule| match rule.holds(value, snap) {
                true => Ok(rule.id()),
                false => Err(AuditError::InvariantFailed(rule.id())),
            })
            .collect()
    }
}
