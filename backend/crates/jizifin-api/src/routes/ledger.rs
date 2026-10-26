//! `/entries`: the unified ledger.

use super::query::today;
use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::{AppState, blocking};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use jizifin_auditor::AuditScope;
use jizifin_auditor::shares::EntryShares;
use jizifin_domain::Month;
use jizifin_domain::entities::Entry;
use jizifin_domain::ids::EntryId;
use jizifin_store::inputs::{DuplicateProbe, EntryInput};
use jizifin_store::repo::{DuplicateMatch, Entries, EntryFilter, EntryPage};
use std::sync::Arc;

/// Ledger routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/entries", get(list).post(create))
        .route("/entries/batch", post(create_batch))
        .route("/entries/duplicates", post(duplicates))
        .route("/entries/{id}", get(fetch).put(update).delete(remove))
        .route("/entries/{id}/shares", get(shares))
}

async fn shares(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<EntryShares>, ApiError> {
    let entry = s.read(move |c| Entries::get(c, EntryId(id))).await?;
    let scope = AuditScope::month(Month::of(entry.date), today());
    let auditor = Arc::clone(&s.auditor);
    Ok(Json(blocking(move || auditor.entry_shares(scope, entry.id)).await?))
}

async fn list(State(s): State<AppState>, Query(filter): Query<EntryFilter>) -> Result<Json<EntryPage>, ApiError> {
    Ok(Json(s.read(move |c| Entries::list(c, &filter)).await?))
}

async fn fetch(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Entry>, ApiError> {
    Ok(Json(s.read(move |c| Entries::get(c, EntryId(id))).await?))
}

async fn create(State(s): State<AppState>, Json(input): Json<EntryInput>) -> Created {
    Ok(created(s.write("entries.created", move |c| Entries::create(c, &input)).await?))
}

async fn create_batch(State(s): State<AppState>, Json(inputs): Json<Vec<EntryInput>>) -> Result<(StatusCode, Json<Vec<EntryId>>), ApiError> {
    let ids = s.write("entries.created", move |c| Entries::create_batch(c, &inputs)).await?;
    Ok((StatusCode::CREATED, Json(ids)))
}

async fn update(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<EntryInput>) -> Done {
    s.write("entries.updated", move |c| Entries::update(c, EntryId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("entries.deleted", move |c| Entries::delete(c, EntryId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn duplicates(State(s): State<AppState>, Json(probes): Json<Vec<DuplicateProbe>>) -> Result<Json<Vec<DuplicateMatch>>, ApiError> {
    Ok(Json(s.read(move |c| Entries::duplicates(c, &probes)).await?))
}
