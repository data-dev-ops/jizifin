//! `/categories`, `/agreements`, `/budgets`, `/tags`.

use super::query::MonthQuery;
use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, put};
use axum::{Json, Router};
use jizifin_domain::Month;
use jizifin_domain::entities::{Agreement, Budget, Category, Tag};
use jizifin_domain::ids::{AgreementId, CategoryId, TagId};
use jizifin_store::inputs::{AgreementInput, CategoryInput, TagInput};
use jizifin_store::repo::{Agreements, Budgets, Categories, Tags};

/// Catalog routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories).post(create_category))
        .route("/categories/{id}", put(update_category).delete(delete_category))
        .route("/categories/{id}/agreements", get(list_agreements).post(create_agreement))
        .route("/agreements/{id}", put(update_agreement).delete(delete_agreement))
        .route("/budgets", get(list_budgets).put(upsert_budget))
        .route("/budgets/{category}/{month}", delete(delete_budget))
        .route("/tags", get(list_tags).post(create_tag))
        .route("/tags/{id}", put(update_tag).delete(delete_tag))
}

async fn list_categories(State(s): State<AppState>) -> Result<Json<Vec<Category>>, ApiError> {
    Ok(Json(s.read(Categories::list).await?))
}

async fn create_category(State(s): State<AppState>, Json(input): Json<CategoryInput>) -> Created {
    Ok(created(s.write("categories.created", move |c| Categories::create(c, &input)).await?))
}

async fn update_category(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<CategoryInput>) -> Done {
    s.write("categories.updated", move |c| Categories::update(c, CategoryId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_category(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("categories.deleted", move |c| Categories::delete(c, CategoryId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_agreements(State(s): State<AppState>, Path(id): Path<i64>) -> Result<Json<Vec<Agreement>>, ApiError> {
    Ok(Json(s.read(move |c| Agreements::for_category(c, CategoryId(id))).await?))
}

async fn create_agreement(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<AgreementInput>) -> Created {
    Ok(created(s.write("agreements.created", move |c| Agreements::create(c, CategoryId(id), &input)).await?))
}

async fn update_agreement(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<AgreementInput>) -> Done {
    s.write("agreements.updated", move |c| Agreements::update(c, AgreementId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_agreement(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("agreements.deleted", move |c| Agreements::delete(c, AgreementId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_budgets(State(s): State<AppState>, Query(q): Query<MonthQuery>) -> Result<Json<Vec<Budget>>, ApiError> {
    let period = q.month()?.period();
    Ok(Json(s.read(move |c| Budgets::list(c, period)).await?))
}

async fn upsert_budget(State(s): State<AppState>, Json(input): Json<Budget>) -> Done {
    s.write("budgets.updated", move |c| Budgets::upsert(c, &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_budget(State(s): State<AppState>, Path((category, month)): Path<(i64, String)>) -> Done {
    let month = Month::parse(&month)?;
    s.write("budgets.deleted", move |c| Budgets::delete(c, CategoryId(category), month)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_tags(State(s): State<AppState>) -> Result<Json<Vec<Tag>>, ApiError> {
    Ok(Json(s.read(Tags::list).await?))
}

async fn create_tag(State(s): State<AppState>, Json(input): Json<TagInput>) -> Created {
    Ok(created(s.write("tags.created", move |c| Tags::create(c, &input)).await?))
}

async fn update_tag(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<TagInput>) -> Done {
    s.write("tags.updated", move |c| Tags::update(c, TagId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_tag(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("tags.deleted", move |c| Tags::delete(c, TagId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}
