//! `/accounts`: joint accounts with roster and categories.

use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use jizifin_domain::entities::Account;
use jizifin_domain::ids::AccountId;
use jizifin_store::inputs::AccountInput;
use jizifin_store::repo::Accounts;

/// Account routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/accounts", get(list).post(create))
        .route("/accounts/{id}", get(fetch).put(update).delete(remove))
}

async fn list(State(s): State<AppState>) -> Result<Json<Vec<Account>>, ApiError> {
    Ok(Json(s.read(Accounts::all).await?))
}

async fn fetch(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Account>, ApiError> {
    Ok(Json(s.read(move |c| Accounts::get(c, AccountId(id))).await?))
}

async fn create(State(s): State<AppState>, Json(input): Json<AccountInput>) -> Created {
    Ok(created(s.write("accounts.created", move |c| Accounts::create(c, &input)).await?))
}

async fn update(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<AccountInput>) -> Done {
    s.write("accounts.updated", move |c| Accounts::update(c, AccountId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("accounts.deleted", move |c| Accounts::delete(c, AccountId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}
