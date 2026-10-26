//! `/categories`, `/agreements`, `/budgets`, `/tags`.

use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use jizifin_domain::entities::{Agreement, Budget, Category, Share, Tag};
use serde::Deserialize;
use jizifin_domain::ids::{AgreementId, BudgetId, CategoryId, TagId};
use jizifin_store::inputs::{AgreementInput, BudgetInput, CategoryInput, TagInput};
use jizifin_store::repo::{Agreements, Budgets, Categories, Tags};

/// Catalog routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/categories", get(list_categories).post(create_category))
        .route("/categories/{id}", put(update_category).delete(delete_category))
        .route("/categories/{id}/agreements", get(list_agreements).post(create_agreement))
        .route("/categories/{id}/baseline", put(set_baseline))
        .route("/agreements/{id}", put(update_agreement).delete(delete_agreement))
        .route("/budgets", get(list_budgets).post(create_budget))
        .route("/budgets/{id}", put(update_budget).delete(delete_budget))
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

/// `{ "shares": [{ member, pct }] }`
#[derive(Deserialize)]
struct BaselineBody {
    shares: Vec<Share>,
}

async fn set_baseline(State(s): State<AppState>, Path(id): Path<i64>, Json(body): Json<BaselineBody>) -> Created {
    Ok(created(s.write("agreements.updated", move |c| Agreements::set_baseline(c, CategoryId(id), &body.shares)).await?))
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

async fn list_budgets(State(s): State<AppState>) -> Result<Json<Vec<Budget>>, ApiError> {
    Ok(Json(s.read(Budgets::all).await?))
}

async fn create_budget(State(s): State<AppState>, Json(input): Json<BudgetInput>) -> Created {
    Ok(created(s.write("budgets.created", move |c| Budgets::create(c, &input)).await?))
}

async fn update_budget(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<BudgetInput>) -> Done {
    s.write("budgets.updated", move |c| Budgets::update(c, BudgetId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_budget(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("budgets.deleted", move |c| Budgets::delete(c, BudgetId(id))).await?;
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
