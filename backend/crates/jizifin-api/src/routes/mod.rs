//! Route table. Paths are mounted without `/api`; Caddy strips the prefix.

mod accounts;
mod audit;
mod catalog;
mod console;
mod ledger;
mod people;
mod projects;
mod query;
mod rules;
mod settlements;

use crate::state::AppState;
use crate::{auth, error::ApiError, events};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, middleware};
use serde::Serialize;
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;

/// `201 {"id": …}`
pub(crate) type Created = Result<(StatusCode, Json<Value>), ApiError>;
/// `204` with no body.
pub(crate) type Done = Result<StatusCode, ApiError>;

/// Builds a `201 Created` response carrying the new id.
pub(crate) fn created(id: impl Serialize) -> (StatusCode, Json<Value>) {
    (StatusCode::CREATED, Json(json!({ "id": id })))
}

/// Builds the full router.
pub fn router(state: AppState) -> Router {
    let public = Router::new().route("/health", get(|| async { "ok" })).merge(auth::routes());
    let protected = Router::new()
        .route("/ws/finance", get(events::socket))
        .merge(audit::routes())
        .merge(ledger::routes())
        .merge(people::routes())
        .merge(catalog::routes())
        .merge(accounts::routes())
        .merge(projects::routes())
        .merge(rules::routes())
        .merge(settlements::routes())
        .merge(console::routes())
        .layer(middleware::from_fn_with_state(state.clone(), auth::require_session));
    public.merge(protected).layer(TraceLayer::new_for_http()).with_state(state)
}
