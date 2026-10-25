//! `/projects`.

use super::{Created, Done, created};
use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, put};
use axum::{Json, Router};
use jizifin_domain::entities::Project;
use jizifin_domain::ids::ProjectId;
use jizifin_store::inputs::ProjectInput;
use jizifin_store::repo::Projects;

/// Project routes (settlement lives with the Auditor routes).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/projects", get(list).post(create))
        .route("/projects/{id}", put(update).delete(remove))
}

async fn list(State(s): State<AppState>) -> Result<Json<Vec<Project>>, ApiError> {
    Ok(Json(s.read(Projects::all).await?))
}

async fn create(State(s): State<AppState>, Json(input): Json<ProjectInput>) -> Created {
    Ok(created(s.write("projects.created", move |c| Projects::create(c, &input)).await?))
}

async fn update(State(s): State<AppState>, Path(id): Path<i64>, Json(input): Json<ProjectInput>) -> Done {
    s.write("projects.updated", move |c| Projects::update(c, ProjectId(id), &input)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove(State(s): State<AppState>, Path(id): Path<i64>) -> Done {
    s.write("projects.deleted", move |c| Projects::delete(c, ProjectId(id))).await?;
    Ok(StatusCode::NO_CONTENT)
}
