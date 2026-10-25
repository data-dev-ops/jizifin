//! API error envelope: `{"detail": "..."}`, read by the frontend `request()` helper.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use jizifin_auditor::AuditError;
use jizifin_domain::DomainError;
use serde_json::json;

/// Error returned by every handler.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Invalid client input.
    #[error("{0}")]
    BadRequest(String),
    /// Resource absent.
    #[error("{0}")]
    NotFound(String),
    /// Unexpected server failure, including failed audit invariants.
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

fn log_failure(detail: &str) {
    tracing::error!("request failed: {detail}");
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Internal(detail) = &self {
            log_failure(detail);
        }
        (self.status(), Json(json!({ "detail": self.to_string() }))).into_response()
    }
}

impl From<AuditError> for ApiError {
    fn from(e: AuditError) -> Self {
        match e {
            AuditError::UnknownAccount(_) => Self::NotFound(e.to_string()),
            AuditError::Load(_) | AuditError::InvariantFailed(_) => Self::Internal(e.to_string()),
        }
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self::BadRequest(e.to_string())
    }
}
