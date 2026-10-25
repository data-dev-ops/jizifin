//! `/recurring` templates and `/offset-rules`.

use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use jizifin_domain::entities::{OffsetRule, RecurringTemplate};
use jizifin_domain::ids::{OffsetRuleId, RecurringId};
use jizifin_store::inputs::{OffsetRuleInput, RecurringInput};
use jizifin_store::repo::{OffsetRules, Recurring};

/// Automation routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/recurring", get(list_recurring).post(create_recurring))
        .route("/recurring/{id}", put(update_recurring).delete(delete_recurring))
        .route("/offset-rules", get(list_offsets).post(create_offset))
        .route("/offset-rules/{id}", delete(delete_offset))
}

async fn list_recurring(State(s): State<AppState>) -> Result<Json<Vec<RecurringTemplate>>, ApiError> {
    Ok(Json(s.read(Recurring::list).await?))
}

async fn create_recurring(State(s): State<AppState>, Json(input): Json<RecurringInput>) -> Created {
    Ok(created(s.write("recurring.created", move |c| Recurring::create(c, &input)).await?))
}

async fn update_recurring(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<RecurringInput>) -> Done {
    s.write("recurring.updated", move |c| Recurring::update(c, RecurringId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_recurring(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("recurring.deleted", move |c| Recurring::delete(c, RecurringId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_offsets(State(s): State<AppState>) -> Result<Json<Vec<OffsetRule>>, ApiError> {
    Ok(Json(s.read(OffsetRules::list).await?))
}

async fn create_offset(State(s): State<AppState>, Json(input): Json<OffsetRuleInput>) -> Created {
    Ok(created(s.write("offset_rules.created", move |c| OffsetRules::create(c, &input)).await?))
}

async fn delete_offset(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("offset_rules.deleted", move |c| OffsetRules::delete(c, OffsetRuleId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}
