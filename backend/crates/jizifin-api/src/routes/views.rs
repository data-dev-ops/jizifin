//! `/views/*`: read-only aggregates for charts and summaries (keys are ids; the client maps labels).

use super::query::{MonthQuery, today};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use jizifin_domain::entities::EntryKind;
use jizifin_domain::ids::TagId;
use jizifin_domain::{Month, Period};
use jizifin_store::repo::{Focus, MonthSlice, MonthView, ProjectProgress, RecurringView, SeriesPoint, TagDetail, TagView, Views};
use serde::Deserialize;

/// Months covered by `/views/series` when no range is given.
const DEFAULT_SERIES_MONTHS: u32 = 12;

/// View routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/views/month", get(month))
        .route("/views/series", get(series))
        .route("/views/tags", get(tags))
        .route("/views/tags/{id}", get(tag))
        .route("/views/projects", get(projects))
        .route("/views/recurring", get(recurring))
}

/// `?kind=expense|income`, defaulting to expense.
#[derive(Debug, Default, Deserialize)]
struct KindQuery {
    kind: Option<EntryKind>,
}

impl KindQuery {
    fn kind(&self) -> EntryKind {
        self.kind.unwrap_or(EntryKind::Expense)
    }
}

/// `?kind=…&from=YYYY-MM&to=YYYY-MM`
#[derive(Debug, Default, Deserialize)]
struct SeriesQuery {
    kind: Option<EntryKind>,
    from: Option<String>,
    to: Option<String>,
}

impl SeriesQuery {
    fn period(&self) -> Result<Period, ApiError> {
        let to = self.to.as_deref().map(Month::parse).transpose()?.unwrap_or_else(|| Month::of(today()));
        let from = match self.from.as_deref() {
            Some(raw) => Month::parse(raw)?,
            None => (1..DEFAULT_SERIES_MONTHS).fold(to, |m, _| m.previous()),
        };
        Ok(Period { from: from.first_day(), to: to.last_day() })
    }
}

/// `?month&members&kind&category&member&project&account`
async fn month(
    State(s): State<AppState>,
    Query(q): Query<MonthQuery>,
    Query(k): Query<KindQuery>,
    Query(focus): Query<Focus>,
) -> Result<Json<MonthView>, ApiError> {
    let slice = MonthSlice { kind: k.kind(), month: q.month()?, members: q.members()?, focus };
    Ok(Json(s.read(move |c| Views::month(c, &slice)).await?))
}

async fn series(State(s): State<AppState>, Query(q): Query<SeriesQuery>) -> Result<Json<Vec<SeriesPoint>>, ApiError> {
    let (kind, period) = (q.kind.unwrap_or(EntryKind::Expense), q.period()?);
    Ok(Json(s.read(move |c| Views::series(c, kind, period)).await?))
}

async fn tags(State(s): State<AppState>) -> Result<Json<Vec<TagView>>, ApiError> {
    Ok(Json(s.read(Views::tags).await?))
}

async fn tag(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<TagDetail>, ApiError> {
    Ok(Json(s.read(move |c| Views::tag(c, TagId(id))).await?))
}

async fn projects(State(s): State<AppState>) -> Result<Json<Vec<ProjectProgress>>, ApiError> {
    let today = today();
    Ok(Json(s.read(move |c| Views::projects(c, today)).await?))
}

async fn recurring(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Result<Json<RecurringView>, ApiError> {
    let month = q.month()?;
    Ok(Json(s.read(move |c| Views::recurring(c, month)).await?))
}
