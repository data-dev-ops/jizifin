//! `/members`, `/jobs`, `/income-overrides`.

use super::query::MonthQuery;
use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use jizifin_domain::Month;
use jizifin_domain::entities::{IncomeOverride, Job, Member};
use jizifin_domain::ids::{JobId, MemberId};
use jizifin_store::inputs::{JobAdjustment, JobInput, MemberInput};
use jizifin_store::repo::{IncomeOverrides, Jobs, Members};

/// People and income routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/members", get(list_members).post(create_member))
        .route("/members/{id}", put(update_member).delete(delete_member))
        .route("/jobs", get(list_jobs).post(create_job))
        .route("/jobs/{id}", put(update_job).delete(delete_job))
        .route("/jobs/{id}/adjust", post(adjust_job))
        .route("/income-overrides", get(list_overrides).put(upsert_override))
        .route("/income-overrides/{member}/{month}", delete(delete_override))
}

async fn list_members(State(s): State<AppState>) -> Result<Json<Vec<Member>>, ApiError> {
    Ok(Json(s.read(Members::list).await?))
}

async fn create_member(State(s): State<AppState>, Json(input): Json<MemberInput>) -> Created {
    Ok(created(s.write("members.created", move |c| Members::create(c, &input)).await?))
}

async fn update_member(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<MemberInput>) -> Done {
    s.write("members.updated", move |c| Members::update(c, MemberId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_member(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("members.deleted", move |c| Members::delete(c, MemberId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_jobs(State(s): State<AppState>) -> Result<Json<Vec<Job>>, ApiError> {
    Ok(Json(s.read(Jobs::list).await?))
}

async fn create_job(State(s): State<AppState>, Json(input): Json<JobInput>) -> Created {
    Ok(created(s.write("jobs.created", move |c| Jobs::create(c, &input)).await?))
}

async fn update_job(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<JobInput>) -> Done {
    s.write("jobs.updated", move |c| Jobs::update(c, JobId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn adjust_job(State(s): State<AppState>, Path(id): Path<i64>, Json(change): Json<JobAdjustment>) -> Created {
    Ok(created(s.write("jobs.updated", move |c| Jobs::adjust(c, JobId(id), &change)).await?))
}

async fn delete_job(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("jobs.deleted", move |c| Jobs::delete(c, JobId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_overrides(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Result<Json<Vec<IncomeOverride>>, ApiError> {
    let period = q.month()?.period();
    Ok(Json(s.read(move |c| IncomeOverrides::list(c, period)).await?))
}

async fn upsert_override(State(s): State<AppState>, Json(input): Json<IncomeOverride>) -> Done {
    s.write("income_overrides.updated", move |c| IncomeOverrides::upsert(c, &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_override(State(s): State<AppState>, Path((member, month)): Path<(i64, String)>) -> Done {
    let month = Month::parse(&month)?;
    s.write("income_overrides.deleted", move |c| IncomeOverrides::delete(c, MemberId(member), month)).await?;
    Ok(StatusCode::NO_CONTENT)
}
