//! Opaque deterministic AES-GCM ciphertext. Supports equality only, never ordering.

use crate::DomainError;
use serde::{Deserialize, Serialize};

/// Base64URL (no padding) ciphertext produced by the client.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Ciphertext(String);

impl Ciphertext {
    /// Validates and wraps a ciphertext string.
    pub fn parse(raw: impl Into<String>) -> Result<Self, DomainError> {
        let raw = raw.into();
        match Self::is_base64url(&raw) {
            true => Ok(Self(raw)),
            false => Err(DomainError::InvalidCiphertext),
        }
    }

    /// Borrowed ciphertext text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_base64url(raw: &str) -> bool {
        !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    }
}

impl TryFrom<String> for Ciphertext {
    type Error = DomainError;
    fn try_from(raw: String) -> Result<Self, DomainError> {
        Self::parse(raw)
    }
}

impl From<Ciphertext> for String {
    fn from(c: Ciphertext) -> Self {
        c.0
    }
}
