//! `/settlements`: live Auditor settlements and sealed months.

use super::query::{MonthQuery, today};
use crate::error::ApiError;
use crate::state::{AppState, blocking};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use jizifin_auditor::auditor::SealedSettlement;
use jizifin_auditor::settlement::Settlement;
use jizifin_auditor::{AuditScope, AuditedResult};
use jizifin_domain::Month;
use jizifin_domain::entities::MonthSeal;
use jizifin_store::repo::Seals;
use serde::Serialize;
use std::sync::Arc;

/// A month is either sealed (authoritative stored result) or computed live.
#[derive(Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum SettlementView {
    Sealed(SealedSettlement),
    Live(AuditedResult<Settlement>),
}

/// Settlement routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settlements", get(list))
        .route("/settlements/{month}", get(fetch))
        .route("/settlements/{month}/seal", post(seal))
}

async fn list(State(s): State<AppState>) -> Result<Json<Vec<MonthSeal>>, ApiError> {
    Ok(Json(s.read(Seals::list).await?))
}

async fn fetch(State(s): State<AppState>, Path(month): Path<String>, Query(q): Query<MonthQuery>) -> Result<Json<SettlementView>, ApiError> {
    let month = Month::parse(&month)?;
    let scope = AuditScope::month(month, today()).with_members(q.members()?);
    let stored = s.read(move |c| Seals::get(c, month)).await?;
    let auditor = Arc::clone(&s.auditor);
    let view = match stored {
        Some(seal) => SettlementView::Sealed(blocking(move || auditor.sealed_settlement(scope, seal)).await?),
        None => SettlementView::Live(blocking(move || auditor.settlement(scope)).await?),
    };
    Ok(Json(view))
}

async fn seal(State(s): State<AppState>, Path(month): Path<String>) -> Result<StatusCode, ApiError> {
    let month = Month::parse(&month)?;
    let auditor = Arc::clone(&s.auditor);
    let audited = blocking(move || auditor.settlement(AuditScope::month(month, today()))).await?;
    let json = serde_json::to_string(&audited.value).map_err(|e| ApiError::Internal(e.to_string()))?;
    s.write("settlements.sealed", move |c| Seals::insert(c, month, &json, &audited.inputs_hash)).await?;
    Ok(StatusCode::CREATED)
}
