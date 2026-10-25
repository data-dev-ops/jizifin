use crate::{Ciphertext, ids::MemberId};
use serde::{Deserialize, Serialize};

/// A household member.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Surrogate key.
    pub id: MemberId,
    /// Encrypted display name.
    pub name: Ciphertext,
    /// Hex colour, e.g. `#3b82f6`.
    pub color: String,
    /// Inactive members keep history but receive no new shares.
    pub is_active: bool,
}
