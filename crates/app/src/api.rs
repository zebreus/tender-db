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
use model::{
    Account, Dashboard, Ingestion, NewToken, NewWebhook, QualityHistory, Tender, Token, Webhook,
};

/// All tenders, newest first.
#[get("/api/tenders")]
pub async fn list_tenders() -> ServerFnResult<Vec<Tender>> {
    let db = store::state().await;
    db.list_tenders(200).await.map_err(ServerFnError::new)
}

/// Everything the dashboard shows. Served from the background refresher's
/// memoized snapshot — a request never scans the store (issue 20 part 3), so
/// public traffic cannot pin a core no matter how cold the page cache is.
#[get("/api/dashboard")]
pub async fn dashboard() -> ServerFnResult<Dashboard> {
    Ok(tender_db::coverage::latest())
}

/// The weekly data-quality headline history (issue 265) — the stored
/// `data-quality-headlines` runs, deserialized, with the newest run's age
/// computed server-side. A point lookup on the reports table: no measurement
/// runs on this path, same rule as `/api/dashboard`. Empty (with no age) until
/// the first weekly run has stored a history — "not measured yet" must render
/// as exactly that, never as zeros (issue 37).
#[get("/api/quality")]
pub async fn quality() -> ServerFnResult<QualityHistory> {
    let db = store::state().await;
    match db.latest_report("data-quality-headlines").await.map_err(ServerFnError::new)? {
        Some((body, computed_at)) => Ok(QualityHistory {
            runs: serde_json::from_str(&body).unwrap_or_default(),
            age_seconds: Some(store::now_unix() - computed_at),
        }),
        None => Ok(QualityHistory::default()),
    }
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

// ------------------------------------------------------------------ webhooks

/// The account's registered webhook endpoints (no secrets).
#[get("/api/webhooks", headers: dioxus::fullstack::HeaderMap)]
pub async fn list_webhooks() -> ServerFnResult<Vec<Webhook>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::webhooks::list(&db, account.id).await.map_err(ServerFnError::new)
}

/// Register an endpoint. The signing secret is in the response once and never
/// again, exactly like an API token.
#[post("/api/webhooks/create", headers: dioxus::fullstack::HeaderMap)]
pub async fn create_webhook(url: String) -> ServerFnResult<NewWebhook> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::webhooks::register(&db, account.id, &url).await.map_err(ServerFnError::new)
}

#[post("/api/webhooks/delete", headers: dioxus::fullstack::HeaderMap)]
pub async fn delete_webhook(id: i64) -> ServerFnResult<Vec<Webhook>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::webhooks::delete(&db, account.id, id).await.map_err(ServerFnError::new)?;
    tender_db::webhooks::list(&db, account.id).await.map_err(ServerFnError::new)
}

#[post("/api/webhooks/disable", headers: dioxus::fullstack::HeaderMap)]
pub async fn disable_webhook(id: i64) -> ServerFnResult<Vec<Webhook>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::webhooks::disable(&db, account.id, id).await.map_err(ServerFnError::new)?;
    tender_db::webhooks::list(&db, account.id).await.map_err(ServerFnError::new)
}

/// Re-enable a disabled endpoint, keeping its slot (delivers the backlog it
/// missed).
#[post("/api/webhooks/enable", headers: dioxus::fullstack::HeaderMap)]
pub async fn enable_webhook(id: i64) -> ServerFnResult<Vec<Webhook>> {
    let db = store::state().await;
    let account = require_account(&db, &headers).await?;
    tender_db::webhooks::enable(&db, account.id, id, false).await.map_err(ServerFnError::new)?;
    tender_db::webhooks::list(&db, account.id).await.map_err(ServerFnError::new)
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
