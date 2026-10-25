//! Calendar value objects: `Month` (YYYY-MM) and closed date `Period`.

use crate::DomainError;
use chrono::{Datelike, Months, NaiveDate};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A calendar month, stored as its first day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Month(NaiveDate);

impl Month {
    /// Parses `YYYY-MM`.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        NaiveDate::parse_from_str(&format!("{raw}-01"), "%Y-%m-%d")
            .ok()
            .filter(|_| raw.len() == 7)
            .map(Self)
            .ok_or_else(|| DomainError::InvalidMonth(raw.to_owned()))
    }

    /// Month containing `date`.
    pub fn of(date: NaiveDate) -> Self {
        Self(date.with_day(1).unwrap_or(date))
    }

    /// First day of the month.
    pub fn first_day(self) -> NaiveDate {
        self.0
    }

    /// Last day of the month.
    pub fn last_day(self) -> NaiveDate {
        self.next().0.pred_opt().unwrap_or(self.0)
    }

    /// Following month.
    pub fn next(self) -> Self {
        Self(self.0 + Months::new(1))
    }

    /// Preceding month.
    pub fn previous(self) -> Self {
        Self(self.0 - Months::new(1))
    }

    /// Closed period covering the whole month.
    pub fn period(self) -> Period {
        Period { from: self.first_day(), to: self.last_day() }
    }

    /// Day number clamped to the month length (e.g. 31 → 30 in April).
    pub fn clamp_day(self, day: u32) -> u32 {
        day.clamp(1, self.last_day().day())
    }
}

impl fmt::Display for Month {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m"))
    }
}

impl TryFrom<String> for Month {
    type Error = DomainError;
    fn try_from(raw: String) -> Result<Self, DomainError> {
        Self::parse(&raw)
    }
}

impl From<Month> for String {
    fn from(m: Month) -> Self {
        m.to_string()
    }
}

/// Closed date range `[from, to]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Period {
    /// First included day.
    pub from: NaiveDate,
    /// Last included day.
    pub to: NaiveDate,
}

impl Period {
    /// Validated constructor.
    pub fn new(from: NaiveDate, to: NaiveDate) -> Result<Self, DomainError> {
        match to < from {
            true => Err(DomainError::InvertedPeriod { from, to }),
            false => Ok(Self { from, to }),
        }
    }

    /// True when `date` lies inside the period.
    pub fn contains(self, date: NaiveDate) -> bool {
        self.from <= date && date <= self.to
    }

    /// True when the optional window `[from, to?]` overlaps this period.
    pub fn overlaps(self, from: NaiveDate, to: Option<NaiveDate>) -> bool {
        from <= self.to && to.is_none_or(|end| end >= self.from)
    }
}
