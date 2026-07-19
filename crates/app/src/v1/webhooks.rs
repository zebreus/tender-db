//! `/v1/webhooks` — account holders register URLs that receive signed change
//! batches (issue 08). CRUD only; the delivery engine is [`crate::webhooks`].
//!
//! Bearer-token gated by the same [`AuthUser`] extractor as the rest of the
//! account-scoped API. Every handler is scoped to the extracted user, so one
//! account can never see or touch another's endpoints.

use crate::v1::{ApiError, AppState, AuthUser};
use crate::webhooks;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::json;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/webhooks", get(list).post(create))
        .route("/v1/webhooks/{id}", get(detail).delete(delete))
        .route("/v1/webhooks/{id}/enable", post(enable))
        .route("/v1/webhooks/{id}/disable", post(disable))
}

#[derive(Deserialize)]
struct CreateRequest {
    url: String,
}

/// `POST /v1/webhooks` — register an endpoint. The signing secret is in the
/// response body once and never again.
async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<axum::Json<CreateRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, ApiError> {
    let axum::Json(req) = body.map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    match webhooks::register(&state.db, user.id(), &req.url).await {
        Ok(created) => Ok((StatusCode::CREATED, axum::Json(created)).into_response()),
        // A rejected URL (bad scheme, private address, unresolvable) is the
        // caller's error, not ours.
        Err(message) => Err(ApiError(StatusCode::BAD_REQUEST, message)),
    }
}

/// `GET /v1/webhooks` — the caller's endpoints (no secrets).
async fn list(State(state): State<AppState>, user: AuthUser) -> Result<Response, ApiError> {
    let webhooks = webhooks::list(&state.db, user.id()).await.map_err(internal)?;
    Ok(axum::Json(json!({ "webhooks": webhooks })).into_response())
}

/// `GET /v1/webhooks/{id}` — one endpoint with its recent delivery attempts.
async fn detail(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    match webhooks::detail(&state.db, user.id(), id).await.map_err(internal)? {
        Some((webhook, deliveries)) => {
            Ok(axum::Json(json!({ "webhook": webhook, "deliveries": deliveries })).into_response())
        }
        None => Err(ApiError::not_found("webhook")),
    }
}

async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    if webhooks::delete(&state.db, user.id(), id).await.map_err(internal)? {
        Ok(StatusCode::NO_CONTENT.into_response())
    } else {
        Err(ApiError::not_found("webhook"))
    }
}

async fn disable(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    if webhooks::disable(&state.db, user.id(), id).await.map_err(internal)? {
        Ok(axum::Json(json!({ "disabled": id })).into_response())
    } else {
        Err(ApiError::not_found("active webhook"))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct EnableRequest {
    /// Resume at the current log head, dropping the backlog accumulated while
    /// disabled. Default false: deliver what was missed.
    from_now: bool,
}

async fn enable(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    body: Option<axum::Json<EnableRequest>>,
) -> Result<Response, ApiError> {
    let from_now = body.map(|axum::Json(r)| r.from_now).unwrap_or(false);
    if webhooks::enable(&state.db, user.id(), id, from_now).await.map_err(internal)? {
        Ok(axum::Json(json!({ "enabled": id })).into_response())
    } else {
        Err(ApiError::not_found("webhook"))
    }
}

fn internal(message: String) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, message)
}
