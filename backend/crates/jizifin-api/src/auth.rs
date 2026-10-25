//! Zero-knowledge login. The client derives a proof from the passphrase; the server
//! stores only its SHA-256 and issues random bearer tokens, also stored hashed.

use crate::error::ApiError;
use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Duration;
use jizifin_store::repo::{Auth, Sessions};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Idle time after which a session expires.
pub const SESSION_TTL: Duration = Duration::days(7);

/// Shortest proof accepted (an AES-GCM ciphertext is far longer).
const MIN_PROOF_LEN: usize = 16;

/// `{ "proof": "<base64url>" }`
#[derive(Deserialize)]
struct ProofBody {
    proof: String,
}

/// `{ "token": "<hex>" }`
#[derive(Serialize)]
struct TokenBody {
    token: String,
}

/// `{ "initialized": bool, "authenticated": bool }`
#[derive(Serialize)]
struct StatusBody {
    initialized: bool,
    authenticated: bool,
}

/// Public `/auth/*` routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/status", get(status))
        .route("/auth/init", post(init))
        .route("/auth/login", post(login))
        .route("/auth/logout", post(logout))
        .route("/auth/reset", post(reset))
}

/// Middleware: rejects requests without a live session.
pub async fn require_session(State(state): State<AppState>, request: Request, next: Next) -> Result<Response, ApiError> {
    let token = bearer(request.headers()).or_else(|| query_token(request.uri().query()));
    match token {
        Some(token) if session_alive(&state, token).await? => Ok(next.run(request).await),
        _ => Err(ApiError::Unauthorized("authentication required".into())),
    }
}

async fn status(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<StatusBody>, ApiError> {
    let initialized = state.read(Auth::proof_hash).await?.is_some();
    let authenticated = match bearer(&headers) {
        Some(token) => session_alive(&state, token).await?,
        None => false,
    };
    Ok(Json(StatusBody { initialized, authenticated }))
}

async fn init(State(state): State<AppState>, Json(body): Json<ProofBody>) -> Result<Json<TokenBody>, ApiError> {
    let hash = proof_hash(&body)?;
    state.write("auth.initialized", move |c| Auth::initialize(c, &hash)).await?;
    issue(&state).await
}

async fn login(State(state): State<AppState>, Json(body): Json<ProofBody>) -> Result<Json<TokenBody>, ApiError> {
    verify(&state, &body).await?;
    issue(&state).await
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Result<Json<serde_json::Value>, ApiError> {
    if let Some(hash) = bearer(&headers).map(sha256_hex) {
        state.write_meta(move |c| Sessions::delete(c, &hash)).await?;
    }
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn reset(State(state): State<AppState>, Json(body): Json<ProofBody>) -> Result<Json<serde_json::Value>, ApiError> {
    verify(&state, &body).await?;
    state.write("auth.reset", Auth::reset).await?;
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

async fn verify(state: &AppState, body: &ProofBody) -> Result<(), ApiError> {
    let given = proof_hash(body)?;
    match state.read(Auth::proof_hash).await? {
        Some(stored) if stored == given => Ok(()),
        Some(_) => Err(ApiError::Unauthorized("incorrect master password".into())),
        None => Err(ApiError::BadRequest("instance not initialized".into())),
    }
}

async fn issue(state: &AppState) -> Result<Json<TokenBody>, ApiError> {
    let token = new_token()?;
    let hash = sha256_hex(&token);
    state.write_meta(move |c| Sessions::create(c, &hash)).await?;
    Ok(Json(TokenBody { token }))
}

async fn session_alive(state: &AppState, token: &str) -> Result<bool, ApiError> {
    let hash = sha256_hex(token);
    state.write_meta(move |c| Sessions::touch(c, &hash, SESSION_TTL)).await
}

fn proof_hash(body: &ProofBody) -> Result<String, ApiError> {
    let valid = body.proof.len() >= MIN_PROOF_LEN && body.proof.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    match valid {
        true => Ok(sha256_hex(&body.proof)),
        false => Err(ApiError::BadRequest("proof must be Base64URL ciphertext".into())),
    }
}

fn new_token() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(hex::encode(bytes))
}

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(str::trim).filter(|t| !t.is_empty())
}

fn query_token(query: Option<&str>) -> Option<&str> {
    query?.split('&').find_map(|pair| pair.strip_prefix("token=")).filter(|t| !t.is_empty())
}
