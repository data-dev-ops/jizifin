//! Route table. Paths are mounted without `/api`; Caddy strips the prefix.

mod audit;
mod query;

use crate::state::AppState;
use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;

/// Builds the full router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .merge(audit::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
