//! Typed surrogate keys.

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id_type {
    ($($name:ident),* $(,)?) => {$(
        #[doc = concat!("Surrogate key for `", stringify!($name), "`.")]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    )*};
}

id_type!(
    MemberId, CategoryId, AgreementId, AccountId, EntryId, RecurringId,
    JobId, ProjectId, TagId, OffsetRuleId,
);
