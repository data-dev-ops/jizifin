//! Domain validation errors.

use thiserror::Error;

/// Raised when a value object or entity is constructed from invalid input.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    /// Month string is not `YYYY-MM`.
    #[error("invalid month '{0}', expected YYYY-MM")]
    InvalidMonth(String),
    /// Ciphertext is empty or not Base64URL without padding.
    #[error("invalid ciphertext: expected Base64URL without padding")]
    InvalidCiphertext,
    /// Period end lies before its start.
    #[error("period end {to} is before start {from}")]
    InvertedPeriod {
        /// Start date.
        from: chrono::NaiveDate,
        /// End date.
        to: chrono::NaiveDate,
    },
    /// Enum text does not match any known variant.
    #[error("unknown {kind} '{value}'")]
    UnknownVariant {
        /// Enum name.
        kind: &'static str,
        /// Offending text.
        value: String,
    },
}
