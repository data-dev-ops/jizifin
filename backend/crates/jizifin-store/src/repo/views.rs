//! Read-only aggregates for charts and summaries. Nothing here is contentious:
//! settlements, deposits, budgets and ratios belong to the Auditor.

use super::Recurring;
use super::progress::{ProjectProgress, project_progress};
use crate::sql::query;
use crate::StoreError;
use chrono::NaiveDate;
use duckdb::{Connection, Row, params};
use jizifin_domain::entities::{EntryKind, RecurringTemplate};
use jizifin_domain::ids::{MemberId, RecurringId, TagId};
use jizifin_domain::{Cents, Month, Period};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Sum and count for one key (category, member, tag or month).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Bucket<K> {
    /// Grouping key.
    pub key: K,
    /// Summed amount.
    pub total: Cents,
    /// Entry count.
    pub count: i64,
    /// Share of the enclosing total, where one applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_pct: Option<f64>,
}

/// One month of one entry kind.
#[derive(Debug, Serialize)]
pub struct MonthView {
    /// Month.
    pub month: Month,
    /// Summed amount.
    pub total: Cents,
    /// Entry count.
    pub count: i64,
    /// Mean entry amount; absent without entries.
    pub average: Option<Cents>,
    /// By category, largest first.
    pub by_category: Vec<Bucket<i64>>,
    /// By member, largest first.
    pub by_member: Vec<Bucket<i64>>,
}

/// One point of a month × category series.
#[derive(Debug, Serialize)]
pub struct SeriesPoint {
    /// Month.
    pub month: Month,
    /// Category id.
    pub category: i64,
    /// Summed amount.
    pub total: Cents,
}

/// Lifetime totals of one tag.
#[derive(Debug, Serialize)]
pub struct TagView {
    /// Tag id.
    pub tag: TagId,
    /// Summed amount.
    pub total: Cents,
    /// Entry count.
    pub count: i64,
    /// First tagged date.
    pub first: Option<NaiveDate>,
    /// Last tagged date.
    pub last: Option<NaiveDate>,
    /// Total spread over the calendar months from first to last entry.
    pub monthly_avg: Cents,
}

/// Lifetime breakdown of one tag.
#[derive(Debug, Serialize)]
pub struct TagDetail {
    /// Totals.
    pub summary: Option<TagView>,
    /// By month, chronological.
    pub by_month: Vec<Bucket<Month>>,
    /// By category, largest first.
    pub by_category: Vec<Bucket<i64>>,
}

/// One template's occurrences in the month.
#[derive(Debug, Serialize)]
pub struct DueTemplate {
    /// Template.
    pub recurring: RecurringId,
    /// Occurrence dates.
    pub dates: Vec<NaiveDate>,
    /// `amount × occurrences`.
    pub total: Cents,
}

/// Recurring commitments projected onto one month.
#[derive(Debug, Serialize)]
pub struct RecurringView {
    /// Month.
    pub month: Month,
    /// Projected total.
    pub total: Cents,
    /// Active templates.
    pub active: usize,
    /// Templates due this month.
    pub due: Vec<DueTemplate>,
    /// By category, largest first.
    pub by_category: Vec<Bucket<i64>>,
    /// By member, largest first.
    pub by_member: Vec<Bucket<i64>>,
}

const BUCKET_TAIL: &str = "CAST(SUM(amount_cents) AS BIGINT), COUNT(*)";
/// `kind = ? AND date in [?, ?] AND (? IS NULL OR member in list ?)` plus one optional equality per [`Focus`] field.
const MONTH_FILTER: &str = "kind = ? AND entry_date BETWEEN ? AND ? \
    AND (? IS NULL OR list_contains(CAST(string_split(?, ',') AS BIGINT[]), member_id)) \
    AND (? IS NULL OR category_id = ?) AND (? IS NULL OR member_id = ?) \
    AND (? IS NULL OR project_id = ?) AND (? IS NULL OR account_id = ?)";

/// One month of one entry kind, optionally for some members and narrowed by a [`Focus`].
#[derive(Clone, Debug)]
pub struct MonthSlice {
    /// Entry kind.
    pub kind: EntryKind,
    /// Month.
    pub month: Month,
    /// Member filter; empty means everyone.
    pub members: BTreeSet<MemberId>,
    /// Extra narrowing.
    pub focus: Focus,
}

/// Optional narrowing of a month view to one category, member, project or account.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct Focus {
    /// Category.
    pub category: Option<i64>,
    /// Payer or earner.
    pub member: Option<i64>,
    /// Project.
    pub project: Option<i64>,
    /// Joint account.
    pub account: Option<i64>,
}

/// Aggregate views over the ledger.
pub struct Views;

impl Views {
    /// Totals of one month slice by category and by member.
    pub fn month(conn: &Connection, slice: &MonthSlice) -> Result<MonthView, StoreError> {
        let month = slice.month;
        let filter = MonthFilter::new(slice);
        let by_category = filter.buckets(conn, "category_id")?;
        let total: Cents = by_category.iter().map(|b| b.total).sum();
        let count: i64 = by_category.iter().map(|b| b.count).sum();
        let average = (count > 0).then(|| Cents::new((total.value() as f64 / count as f64).round_ties_even() as i64));
        let by_member = with_shares(filter.buckets(conn, "member_id")?, total);
        Ok(MonthView { month, total, count, average, by_category: with_shares(by_category, total), by_member })
    }

    /// Month × category totals of `kind` over `period`.
    pub fn series(conn: &Connection, kind: EntryKind, period: Period) -> Result<Vec<SeriesPoint>, StoreError> {
        let sql = "SELECT date_trunc('month', entry_date)::DATE AS m, category_id, CAST(SUM(amount_cents) AS BIGINT) \
            FROM entries WHERE kind = ? AND entry_date BETWEEN ? AND ? GROUP BY m, category_id ORDER BY m, category_id";
        query(conn, sql, params![kind.label(), period.from, period.to], series_point)
    }

    /// Lifetime totals per tag.
    pub fn tags(conn: &Connection) -> Result<Vec<TagView>, StoreError> {
        let sql = "SELECT tag_id, CAST(SUM(amount_cents) AS BIGINT), COUNT(*), MIN(entry_date), MAX(entry_date), \
            CAST(round(SUM(amount_cents) / (datediff('month', MIN(entry_date), MAX(entry_date)) + 1)) AS BIGINT) \
            FROM entries WHERE tag_id IS NOT NULL GROUP BY tag_id ORDER BY tag_id";
        query(conn, sql, [], tag_view)
    }

    /// Lifetime breakdown of tag `id`.
    pub fn tag(conn: &Connection, id: TagId) -> Result<TagDetail, StoreError> {
        let summary = Self::tags(conn)?.into_iter().find(|t| t.tag == id);
        let month_sql = format!("SELECT date_trunc('month', entry_date)::DATE AS k, {BUCKET_TAIL} FROM entries WHERE tag_id = ? GROUP BY k ORDER BY k");
        let by_month = query(conn, &month_sql, [id.0], month_bucket)?;
        let category_sql = format!("SELECT category_id AS k, {BUCKET_TAIL} FROM entries WHERE tag_id = ? GROUP BY k ORDER BY 2 DESC, k");
        let by_category = query(conn, &category_sql, [id.0], id_bucket)?;
        Ok(TagDetail { summary, by_month, by_category })
    }

    /// Funding progress and forecast per project as of `today`.
    pub fn projects(conn: &Connection, today: NaiveDate) -> Result<Vec<ProjectProgress>, StoreError> {
        project_progress(conn, today)
    }

    /// Active recurring templates projected onto `month`.
    pub fn recurring(conn: &Connection, month: Month) -> Result<RecurringView, StoreError> {
        let active: Vec<RecurringTemplate> = Recurring::list(conn)?.into_iter().filter(|t| t.is_active).collect();
        let due: Vec<(&RecurringTemplate, i64)> = active.iter().map(|t| (t, occurrence_count(t, month))).filter(|(_, n)| *n > 0).collect();
        Ok(RecurringView {
            month,
            total: due.iter().map(|(t, n)| Cents::new(t.amount.value() * *n)).sum(),
            active: active.len(),
            due: due.iter().map(|(t, n)| DueTemplate { recurring: t.id, dates: t.occurrences(month), total: Cents::new(t.amount.value() * *n) }).collect(),
            by_category: tally(&due, |t| t.category.0),
            by_member: tally(&due, |t| t.member.0),
        })
    }
}

/// Bound month filter shared by the bucket queries.
struct MonthFilter {
    kind: &'static str,
    period: Period,
    members: Option<String>,
    focus: Focus,
}

impl MonthFilter {
    fn new(slice: &MonthSlice) -> Self {
        let list: Vec<String> = slice.members.iter().map(|m| m.0.to_string()).collect();
        let members = (!list.is_empty()).then(|| list.join(","));
        Self { kind: slice.kind.label(), period: slice.month.period(), members, focus: slice.focus }
    }

    fn buckets(&self, conn: &Connection, column: &'static str) -> Result<Vec<Bucket<i64>>, StoreError> {
        let sql = format!("SELECT {column} AS k, {BUCKET_TAIL} FROM entries WHERE {MONTH_FILTER} GROUP BY k ORDER BY 2 DESC, k");
        let f = self.focus;
        let p = params![
            self.kind, self.period.from, self.period.to, self.members, self.members,
            f.category, f.category, f.member, f.member, f.project, f.project, f.account, f.account
        ];
        query(conn, &sql, p, id_bucket)
    }
}

fn occurrence_count(template: &RecurringTemplate, month: Month) -> i64 {
    i64::try_from(template.occurrences(month).len()).unwrap_or(i64::MAX)
}

/// Sums projected occurrences by `key`, largest first.
fn tally(due: &[(&RecurringTemplate, i64)], key: impl Fn(&RecurringTemplate) -> i64) -> Vec<Bucket<i64>> {
    let mut sums: BTreeMap<i64, (Cents, i64)> = BTreeMap::new();
    for (template, n) in due {
        let slot = sums.entry(key(template)).or_insert((Cents::new(0), 0));
        *slot = (slot.0 + Cents::new(template.amount.value() * *n), slot.1 + n);
    }
    let mut buckets: Vec<Bucket<i64>> = sums.into_iter().map(|(key, (total, count))| Bucket { key, total, count, share_pct: None }).collect();
    buckets.sort_by(|a, b| b.total.cmp(&a.total).then(a.key.cmp(&b.key)));
    buckets
}

/// Sets each bucket's share of `total` (none when the total is zero).
fn with_shares<K>(buckets: Vec<Bucket<K>>, total: Cents) -> Vec<Bucket<K>> {
    let share = |b: &Bucket<K>| (!total.is_zero()).then(|| b.total.value() as f64 / total.value() as f64 * 100.0);
    buckets.into_iter().map(|b| Bucket { share_pct: share(&b), ..b }).collect()
}

/// Rows without a grouping id (e.g. deposits have no category) share the key 0.
fn id_bucket(r: &Row<'_>) -> duckdb::Result<Bucket<i64>> {
    Ok(Bucket { key: r.get::<_, Option<i64>>(0)?.unwrap_or_default(), total: Cents::new(r.get(1)?), count: r.get(2)?, share_pct: None })
}

fn month_bucket(r: &Row<'_>) -> duckdb::Result<Bucket<Month>> {
    Ok(Bucket { key: Month::of(r.get(0)?), total: Cents::new(r.get(1)?), count: r.get(2)?, share_pct: None })
}

fn series_point(r: &Row<'_>) -> duckdb::Result<SeriesPoint> {
    Ok(SeriesPoint { month: Month::of(r.get(0)?), category: r.get(1)?, total: Cents::new(r.get(2)?) })
}

fn tag_view(r: &Row<'_>) -> duckdb::Result<TagView> {
    Ok(TagView {
        tag: TagId(r.get(0)?),
        total: Cents::new(r.get(1)?),
        count: r.get(2)?,
        first: r.get(3)?,
        last: r.get(4)?,
        monthly_avg: Cents::new(r.get(5)?),
    })
}
