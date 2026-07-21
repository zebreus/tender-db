//! Bearer-token authentication for the public API.
//!
//! An extractor rather than a route-blanket middleware (docs/research/
//! api-layer.md §5): most of `/v1` is public and a few endpoints are gated, so
//! the gate belongs in the handler signature where it is visible, not in
//! routing bookkeeping that has to be kept in sync with which routes are which.
//!
//! ```ignore
//! async fn gated(user: AuthUser, …)          // 401 without a valid token
//! async fn either(user: Option<AuthUser>, …) // anonymous allowed, identified if possible
//! ```
//!
//! Issues 07 (SQL endpoint) and 08 (webhooks) are the intended callers.

use crate::v1::{ApiError, AppState};
use axum::extract::FromRequestParts;
use axum::http::{StatusCode, header, request::Parts};

/// The account behind a request's `Authorization: Bearer tdb_…` header.
///
/// Extraction is a SHA-256 of the presented token and an exact-match lookup —
/// so a wrong token is indistinguishable from an unknown one, in constant time
/// on our side.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthUser(pub store::User);

impl AuthUser {
    pub fn id(&self) -> i64 {
        self.0.id
    }
}

fn bearer(parts: &Parts) -> Option<&str> {
    parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())?
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<AuthUser, ApiError> {
        let Some(token) = bearer(parts) else {
            return Err(unauthorized("send an API token as `Authorization: Bearer tdb_…`"));
        };
        match state.db.authenticate_token(token, store::now_unix()).await? {
            Some(user) => Ok(AuthUser(user)),
            None => Err(unauthorized("that API token is unknown or revoked")),
        }
    }
}

/// `Option<AuthUser>` for endpoints that serve anonymous callers but want the
/// identity when there is one. A *malformed* token is still just "anonymous"
/// here — an endpoint that does not require auth should not start rejecting
/// requests because of a header it did not need.
impl axum::extract::OptionalFromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Option<AuthUser>, ApiError> {
        let Some(token) = bearer(parts) else { return Ok(None) };
        Ok(state.db.authenticate_token(token, store::now_unix()).await?.map(AuthUser))
    }
}

fn unauthorized(message: &str) -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, message.to_owned())
}
