use super::{label_of, parse_variant};
use crate::ids::{AccountId, CategoryId, MemberId, RecurringId};
use crate::{Cents, Ciphertext, DomainError, Month};
use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// Repetition interval of a recurring template.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recurrence {
    /// Every 7 days from the start date.
    Weekly,
    /// Every 14 days.
    Biweekly,
    /// Every 28 days.
    FourWeekly,
    /// Once a month on the due day.
    Monthly,
    /// Every third month from the start month.
    Quarterly,
    /// Once a year in the start month.
    Annual,
}

impl Recurrence {
    const TABLE: [(&'static str, Self); 6] = [
        ("weekly", Self::Weekly),
        ("biweekly", Self::Biweekly),
        ("four_weekly", Self::FourWeekly),
        ("monthly", Self::Monthly),
        ("quarterly", Self::Quarterly),
        ("annual", Self::Annual),
    ];

    /// Parses the stored label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        parse_variant("recurrence", raw, &Self::TABLE)
    }

    /// Stored label.
    pub fn label(self) -> &'static str {
        label_of(self, &Self::TABLE)
    }

    fn interval_days(self) -> Option<i64> {
        match self {
            Self::Weekly => Some(7),
            Self::Biweekly => Some(14),
            Self::FourWeekly => Some(28),
            Self::Monthly | Self::Quarterly | Self::Annual => None,
        }
    }

    fn month_step(self) -> i32 {
        match self {
            Self::Quarterly => 3,
            Self::Annual => 12,
            _ => 1,
        }
    }
}

/// Template that materializes ledger entries on a schedule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecurringTemplate {
    /// Surrogate key.
    pub id: RecurringId,
    /// Expense or income.
    pub kind: super::EntryKind,
    /// Amount per occurrence.
    pub amount: Cents,
    /// Payer or earner.
    pub member: MemberId,
    /// Category.
    pub category: CategoryId,
    /// Joint account charged, if any.
    pub account: Option<AccountId>,
    /// Due day for month-based recurrences (clamped to month length).
    pub day_of_month: u32,
    /// Interval.
    pub recurrence: Recurrence,
    /// First possible occurrence.
    pub valid_from: NaiveDate,
    /// Last possible occurrence.
    pub valid_to: Option<NaiveDate>,
    /// Encrypted description copied to generated entries.
    pub name: Ciphertext,
    /// Paused templates generate nothing.
    pub is_active: bool,
}

impl RecurringTemplate {
    /// Occurrence dates inside `month`, bounded by the validity window.
    pub fn occurrences(&self, month: Month) -> Vec<NaiveDate> {
        let candidates = match self.recurrence.interval_days() {
            Some(days) => self.day_interval_dates(month, days),
            None => self.month_interval_date(month).into_iter().collect(),
        };
        candidates.into_iter().filter(|d| self.in_window(*d)).collect()
    }

    fn in_window(&self, date: NaiveDate) -> bool {
        self.valid_from <= date && self.valid_to.is_none_or(|end| date <= end)
    }

    fn day_interval_dates(&self, month: Month, days: i64) -> Vec<NaiveDate> {
        let first = self.first_on_or_after(month.first_day(), days);
        let step = Duration::days(days);
        std::iter::successors(Some(first), |d| Some(*d + step)).take_while(|d| *d <= month.last_day()).collect()
    }

    fn first_on_or_after(&self, from: NaiveDate, days: i64) -> NaiveDate {
        let behind = (from - self.valid_from).num_days().max(0);
        let steps = (behind + days - 1).div_euclid(days);
        self.valid_from + Duration::days(steps * days)
    }

    fn month_interval_date(&self, month: Month) -> Option<NaiveDate> {
        let elapsed = Self::month_index(month.first_day()) - Self::month_index(self.valid_from);
        let due = elapsed >= 0 && elapsed % self.recurrence.month_step() == 0;
        due.then(|| month.first_day().with_day(month.clamp_day(self.day_of_month))).flatten()
    }

    fn month_index(date: NaiveDate) -> i32 {
        date.year() * 12 + date.month0() as i32
    }
}
