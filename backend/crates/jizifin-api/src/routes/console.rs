//! `POST /query`: read-only SQL console.

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use jizifin_store::repo::{Console, QueryResult};
use serde::Deserialize;

/// `{ "sql": "SELECT …" }`
#[derive(Deserialize)]
struct QueryBody {
    sql: String,
}

/// Console route.
pub fn routes() -> Router<AppState> {
    Router::new().route("/query", post(run))
}

async fn run(State(s): State<AppState>, Json(body): Json<QueryBody>) -> Result<Json<QueryResult>, ApiError> {
    Ok(Json(s.read(move |c| Console::run(c, &body.sql)).await?))
}
