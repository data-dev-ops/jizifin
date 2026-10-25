//! `GET /backup` downloads, `POST /backup` restores a ciphertext-only DuckDB backup file.

use crate::error::ApiError;
use crate::state::{AppState, blocking};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use jizifin_store::{Backup, StoreError};
use std::path::PathBuf;

/// Largest backup accepted for restore.
const MAX_UPLOAD: usize = 512 * 1024 * 1024;

/// Backup routes.
pub fn routes() -> Router<AppState> {
    Router::new().route("/backup", get(download).post(restore).layer(DefaultBodyLimit::max(MAX_UPLOAD)))
}

async fn download(State(s): State<AppState>) -> Result<Response, ApiError> {
    let path = scratch_file(&s)?;
    let (db, out) = (s.db.clone(), path.clone());
    let bytes = blocking(move || Backup::export(&db, &out).and_then(|()| read_and_remove(&out))).await;
    let _ = std::fs::remove_file(&path);
    Ok(attachment(bytes?))
}

async fn restore(State(s): State<AppState>, body: Bytes) -> Result<StatusCode, ApiError> {
    let path = scratch_file(&s)?;
    std::fs::write(&path, &body).map_err(io_error)?;
    let (db, input) = (s.db.clone(), path.clone());
    let result = blocking(move || Backup::restore(&db, &input)).await;
    let _ = std::fs::remove_file(&path);
    result?;
    s.events.publish("backup.restored");
    Ok(StatusCode::NO_CONTENT)
}

fn scratch_file(s: &AppState) -> Result<PathBuf, ApiError> {
    std::fs::create_dir_all(&s.tmp_dir).map_err(io_error)?;
    let mut nonce = [0u8; 8];
    getrandom::fill(&mut nonce).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(s.tmp_dir.join(format!("backup-{}.duckdb", hex::encode(nonce))))
}

fn read_and_remove(path: &std::path::Path) -> Result<Vec<u8>, StoreError> {
    let bytes = std::fs::read(path).map_err(|e| StoreError::Invalid(e.to_string()))?;
    let _ = std::fs::remove_file(path);
    Ok(bytes)
}

fn attachment(bytes: Vec<u8>) -> Response {
    let name = format!("attachment; filename=\"jizifin-{}.duckdb\"", Utc::now().format("%Y-%m-%d"));
    let disposition = HeaderValue::from_str(&name).unwrap_or(HeaderValue::from_static("attachment"));
    let headers = [(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream")), (header::CONTENT_DISPOSITION, disposition)];
    (headers, bytes).into_response()
}

fn io_error(e: std::io::Error) -> ApiError {
    ApiError::Internal(e.to_string())
}
