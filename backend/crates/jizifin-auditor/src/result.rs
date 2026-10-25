//! Envelope returned for every audited value.

use serde::Serialize;

/// A verified value plus provenance.
#[derive(Clone, Debug, Serialize)]
pub struct AuditedResult<T> {
    /// The computed value.
    pub value: T,
    /// Hex SHA-256 of the snapshot it was computed from.
    pub inputs_hash: String,
    /// Store data version at load time.
    pub data_version: u64,
    /// Invariants that were checked and held.
    pub checks: Vec<&'static str>,
}
