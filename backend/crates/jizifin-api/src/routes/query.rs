//! Shared query-string parsing for audited endpoints.

use crate::error::ApiError;
use chrono::{NaiveDate, Utc};
use jizifin_domain::Month;
use jizifin_domain::ids::MemberId;
use serde::Deserialize;
use std::collections::BTreeSet;

/// `?month=YYYY-MM&members=1,2`
#[derive(Debug, Default, Deserialize)]
pub struct MonthQuery {
    month: Option<String>,
    members: Option<String>,
}

impl MonthQuery {
    /// Requested month, defaulting to the current one.
    pub fn month(&self) -> Result<Month, ApiError> {
        match &self.month {
            Some(raw) => Ok(Month::parse(raw)?),
            None => Ok(Month::of(today())),
        }
    }

    /// Requested member filter; empty when absent.
    pub fn members(&self) -> Result<BTreeSet<MemberId>, ApiError> {
        let raw = self.members.as_deref().unwrap_or_default();
        raw.split(',').filter(|s| !s.trim().is_empty()).map(Self::member_id).collect()
    }

    fn member_id(raw: &str) -> Result<MemberId, ApiError> {
        raw.trim().parse().map(MemberId).map_err(|_| ApiError::BadRequest(format!("invalid member id '{raw}'")))
    }
}

/// Evaluation date for status calculations.
pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}
