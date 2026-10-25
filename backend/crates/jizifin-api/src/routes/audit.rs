//! Endpoints answered by the Auditor. Each one builds a scope and runs the audit off the async runtime.

use super::query::{MonthQuery, today};
use crate::error::ApiError;
use crate::state::{AppState, blocking};
use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use jizifin_auditor::budget::PROPOSAL_LOOKBACK_MONTHS;
use jizifin_auditor::{AuditError, AuditScope, AuditedResult, Auditor};
use jizifin_domain::ids::{AccountId, ProjectId};
use serde::Serialize;
use std::sync::Arc;

type Audited<T> = Result<Json<AuditedResult<T>>, ApiError>;

/// Auditor-owned routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects/{id}/settlement", get(project_settlement))
        .route("/accounts/{id}/deposit-proposal", get(deposit_proposal))
        .route("/accounts/{id}/dashboard", get(joint_dashboard))
        .route("/budgets/status", get(budget_status))
        .route("/budgets/proposals", get(budget_proposals))
        .route("/splits/salary-ratios", get(salary_ratios))
        .route("/income/effective", get(effective_income))
}

async fn project_settlement(State(s): State<AppState>, Path(id): Path<i64>) -> Audited<impl Serialize> {
    let scope = AuditScope::project(ProjectId(id), today());
    run(&s, move |a| a.project_settlement(scope, ProjectId(id))).await
}

async fn deposit_proposal(State(s): State<AppState>, Path(id): Path<i64>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::month(q.month()?, today());
    run(&s, move |a| a.deposit_proposal(scope, AccountId(id))).await
}

async fn joint_dashboard(State(s): State<AppState>, Path(id): Path<i64>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::account_to_date(q.month()?, AccountId(id), today());
    run(&s, move |a| a.joint_dashboard(scope, AccountId(id))).await
}

async fn budget_status(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::month(q.month()?, today());
    run(&s, move |a| a.budget_status(scope)).await
}

async fn budget_proposals(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::trailing(q.month()?, PROPOSAL_LOOKBACK_MONTHS, today());
    run(&s, move |a| a.budget_proposals(scope)).await
}

async fn salary_ratios(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::month(q.month()?, today()).with_members(q.members()?);
    run(&s, move |a| a.salary_ratios(scope)).await
}

async fn effective_income(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Audited<impl Serialize> {
    let scope = AuditScope::month(q.month()?, today());
    run(&s, move |a| a.income(scope)).await
}

async fn run<T, F>(state: &AppState, audit: F) -> Audited<T>
where
    T: Send + 'static,
    F: FnOnce(&Auditor) -> Result<AuditedResult<T>, AuditError> + Send + 'static,
{
    let auditor = Arc::clone(&state.auditor);
    Ok(Json(blocking(move || audit(&auditor)).await?))
}
