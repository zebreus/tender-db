//! Public server functions — the client/server boundary.
//!
//! Each `#[get]`/`#[post]` compiles to an Axum handler on the server and a fetch
//! stub on the WASM client. Everything here is deliberately thin: the account
//! lifecycle lives in [`tender_db::accounts`] and the dashboard's measurements
//! in [`tender_db::coverage`], so these functions only move cookies and errors
//! across the boundary. That is also what lets the integration test exercise the
//! lifecycle without a browser bundle.

use dioxus::fullstack::{Json, SetCookie, SetHeader};
use dioxus::prelude::*;
use model::{Account, Dashboard, Ingestion, NewToken, Tender, Token};

/// All tenders, newest first.
#[get("/api/tenders")]
pub async fn list_tenders() -> ServerFnResult<Vec<Tender>> {
    let db = store::state().await;
    db.list_tenders(200).await.map_err(ServerFnError::new)
}

/// Everything the dashboard shows, measured in one pass.
#[get("/api/dashboard")]
pub async fn dashboard() -> ServerFnResult<Dashboard> {
    let db = store::state().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    tender_db::coverage::measure(&db, now).await.map_err(ServerFnError::new)
}

/// The ingestion Supervisor's live state — current job + queue + recent runs.
/// A read-only view: admin *actions* are API-only (the dashboard is public and
/// must never carry the operator secret). When the supervisor has not started
/// (a server render before startup completed), this is the empty default.
#[get("/api/ingestion")]
pub async fn ingestion() -> ServerFnResult<Ingestion> {
    match tender_db::supervisor::get() {
        Some(sup) => sup.ingestion().await.map_err(ServerFnError::new),
        None => Ok(Ingestion::default()),
    }
}

// ------------------------------------------------------------------ accounts

/// The response shape of the two endpoints that open a session: the account,
/// plus the `Set-Cookie` the browser stores and never lets script see.
type SignedIn = (SetHeader<SetCookie>, Json<Account>);

/// Register and sign in. The "lost password = lost account" notice is shown by
/// the form before this is ever called — there is no email here to recover with.
#[post("/api/account/register")]
pub async fn register(username: String, password: String) -> ServerFnResult<SignedIn> {
    let db = store::state().await;
    let (account, session) =
        tender_db::accounts::register(&db, &username, &password).await.map_err(ServerFnError::new)?;
    Ok((set_cookie(tender_db::accounts::session_cookie(&session))?, Json(account)))
}

#[post("/api/account/login")]
pub async fn login(username: String, password: String) -> ServerFnResult<SignedIn> {
    let db = store::state().await;
    let (account, session) =
        tender_db::accounts::login(&db, &username, &password).await.map_err(ServerFnError::new)?;
    Ok((set_cookie(tender_db::accounts::session_cookie(&session))?, Json(account)))
}

#[post("/api/account/logout", headers: dioxus::fullstack::HeaderMap)]
pub async fn logout() -> ServerFnResult<(SetHeader<SetCookie>, Json<()>)> {
    if let Some(session) = session_id(&headers) {
        let db = store::state().await;
        tender_db::accounts::logout(&db, &session).await.map_err(ServerFnError::new)?;
    }
    Ok((set_cookie(tender_db::accounts::cleared_cookie())?, Json(())))
}

/// Who the session cookie says we are — `None` when signed out. Not an error:
/// "signed out" is the ordinary state of the dashboard.
#[get("/api/account/me", headers: dioxus::fullstack::HeaderMap)]
pub async fn me() -> ServerFnResult<Option<Account>> {
    let db = store::state().await;
    signed_in(&db, &headers).await
}

/// Mint an API token. Its plaintext is in this response and nowhere else, ever.
#[post("/api/account/tokens/create", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_token(name: String) -> ServerFnResult<NewToken> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::accounts::create_token(&db, account.id, &name).await.map_err(ServerFnError::new)
}

#[get("/api/account/tokens", headers: dioxus::fullstack::HeaderMap)]
pub async fn list_tokens() -> ServerFnResult<Vec<Token>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::accounts::list_tokens(&db, account.id).await.map_err(ServerFnError::new)
}

#[post("/api/account/tokens/revoke", headers: dioxus::fullstack::HeaderMap)]
pub async fn revoke_token(token_id: i64) -> ServerFnResult<Vec<Token>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::accounts::revoke_token(&db, account.id, token_id)
        .await
        .map_err(ServerFnError::new)?;
    tender_db::accounts::list_tokens(&db, account.id).await.map_err(ServerFnError::new)
}

/// Delete the account and everything that authenticates as it, then clear the
/// cookie — the session it names no longer exists either way.
#[post("/api/account/delete", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_account() -> ServerFnResult<(SetHeader<SetCookie>, Json<()>)> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::accounts::delete_account(&db, account.id).await.map_err(ServerFnError::new)?;
    Ok((set_cookie(tender_db::accounts::cleared_cookie())?, Json(())))
}

// ------------------------------------------------------------------- helpers

#[cfg(feature = "server")]
fn set_cookie(value: String) -> ServerFnResult<SetHeader<SetCookie>> {
    SetHeader::<SetCookie>::new(value).map_err(ServerFnError::new)
}

#[cfg(feature = "server")]
fn session_id(headers: &dioxus::fullstack::HeaderMap) -> Option<String> {
    headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())
        .and_then(tender_db::accounts::session_from_cookies)
}

#[cfg(feature = "server")]
async fn signed_in(
    db: &store::Db,
    headers: &dioxus::fullstack::HeaderMap,
) -> ServerFnResult<Option<Account>> {
    match session_id(headers) {
        Some(session) => {
            tender_db::accounts::session_account(db, &session).await.map_err(ServerFnError::new)
        }
        None => Ok(None),
    }
}

#[cfg(feature = "server")]
async fn require_account(
    db: &store::Db,
    headers: &dioxus::fullstack::HeaderMap,
) -> ServerFnResult<Account> {
    signed_in(db, headers)
        .await?
        .ok_or_else(|| ServerFnError::new(tender_db::accounts::AuthError::NotSignedIn))
}
