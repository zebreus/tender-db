//! Issue 06 — the account lifecycle end to end.
//!
//! The lifecycle functions the dashboard's server functions wrap, over a real
//! Turso file, plus the one thing that can only be checked on the wire: that a
//! minted token opens the gated `/v1/me` and a revoked one does not. The server
//! functions themselves add only cookie plumbing, which is unit-tested where it
//! lives.
#![cfg(feature = "server")]

use std::sync::Arc;
use store::Db;
use tender_db::{accounts, v1};

struct Server {
    db: Arc<Db>,
    base: String,
    http: reqwest::Client,
    path: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Server {
    async fn start(name: &str) -> Server {
        let path = format!("/tmp/tender-db-accounts-{name}-{}.db", std::process::id());
        let _ = std::fs::remove_file(&path);
        let db = Arc::new(Db::open(&path).await.expect("open scratch db"));

        let state = v1::AppState::new(db.clone(), db.readers(2).expect("readers"));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            let _ = axum::serve(listener, v1::router(state)).await;
        });

        Server { db, base: format!("http://127.0.0.1:{port}"), http: reqwest::Client::new(), path }
    }

    /// `GET /v1/me` with an optional bearer token — the gated probe.
    async fn me(&self, token: Option<&str>) -> reqwest::Response {
        let mut request = self.http.get(format!("{}/v1/me", self.base));
        if let Some(token) = token {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        request.send().await.expect("request")
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn register_login_token_revoke() {
    let server = Server::start("lifecycle").await;
    let db = &server.db;

    // The gate is shut before anything exists, and says so as 401 rather than
    // 404 or 500.
    assert_eq!(server.me(None).await.status(), 401);
    assert_eq!(server.me(Some("tdb_nonsense")).await.status(), 401);

    // ---- register -------------------------------------------------------
    let (account, session) =
        accounts::register(db, "ada", "correct horse battery").await.expect("register");
    assert_eq!(account.username, "ada");
    // Registration signs you in: the session it returned resolves to the account.
    assert_eq!(accounts::session_account(db, &session).await.expect("session"), Some(account.clone()));

    // The username is now taken, and the policies hold.
    assert!(accounts::register(db, "ada", "correct horse battery").await.is_err());
    assert!(accounts::register(db, "Ada", "correct horse battery").await.is_err());
    assert!(accounts::register(db, "bob", "short").await.is_err());

    // ---- login ----------------------------------------------------------
    assert!(accounts::login(db, "ada", "wrong password!!").await.is_err());
    assert!(accounts::login(db, "nobody", "correct horse battery").await.is_err());
    let (same, second_session) =
        accounts::login(db, "ada", "correct horse battery").await.expect("login");
    assert_eq!(same, account);
    // Logging in a second time does not disturb the first session.
    assert_ne!(second_session, session);
    assert!(accounts::session_account(db, &session).await.expect("session").is_some());

    // ---- token → /v1/me -------------------------------------------------
    let minted = accounts::create_token(db, account.id, "ci").await.expect("mint");
    assert!(minted.token.starts_with("tdb_"));
    assert_eq!(minted.record.revoked_at, None);

    let response = server.me(Some(&minted.token)).await;
    assert_eq!(response.status(), 200);
    let body: serde_json::Value = response.json().await.expect("json");
    assert_eq!(body["user"]["id"], account.id);
    assert_eq!(body["user"]["username"], "ada");

    // Using it recorded the use — that is what makes a stale token visible.
    let listed = accounts::list_tokens(db, account.id).await.expect("list");
    assert_eq!(listed.len(), 1);
    assert!(listed[0].last_used_at.is_some());

    // ---- revoke ---------------------------------------------------------
    assert!(accounts::revoke_token(db, account.id, minted.record.id).await.expect("revoke"));
    assert_eq!(server.me(Some(&minted.token)).await.status(), 401);
    // Revoking is idempotent and the token stays listed, now marked.
    assert!(!accounts::revoke_token(db, account.id, minted.record.id).await.expect("revoke"));
    assert!(accounts::list_tokens(db, account.id).await.expect("list")[0].revoked_at.is_some());

    // ---- logout ---------------------------------------------------------
    accounts::logout(db, &session).await.expect("logout");
    assert!(accounts::session_account(db, &session).await.expect("session").is_none());
    // Only that session ended.
    assert!(accounts::session_account(db, &second_session).await.expect("session").is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn deleting_an_account_closes_the_gate() {
    let server = Server::start("delete").await;
    let db = &server.db;

    let (account, session) = accounts::register(db, "grace", "another long one").await.expect("register");
    let minted = accounts::create_token(db, account.id, "laptop").await.expect("mint");
    assert_eq!(server.me(Some(&minted.token)).await.status(), 200);

    accounts::delete_account(db, account.id).await.expect("delete");

    // Both credentials die with the account.
    assert_eq!(server.me(Some(&minted.token)).await.status(), 401);
    assert!(accounts::session_account(db, &session).await.expect("session").is_none());
    // And the username is free to register again — nothing of it survives.
    assert!(accounts::register(db, "grace", "another long one").await.is_ok());
}

#[tokio::test(flavor = "multi_thread")]
async fn one_account_cannot_touch_another() {
    let server = Server::start("isolation").await;
    let db = &server.db;

    let (ada, _) = accounts::register(db, "ada", "correct horse battery").await.expect("register");
    let (eve, _) = accounts::register(db, "eve", "correct horse battery").await.expect("register");

    let ada_token = accounts::create_token(db, ada.id, "ci").await.expect("mint");
    assert!(!accounts::revoke_token(db, eve.id, ada_token.record.id).await.expect("revoke"));
    assert_eq!(server.me(Some(&ada_token.token)).await.status(), 200);
    assert!(accounts::list_tokens(db, eve.id).await.expect("list").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_dashboard_measures_an_empty_database_honestly() {
    let server = Server::start("dashboard").await;
    let now = 1_800_000_000;

    let dashboard = tender_db::coverage::measure(&server.db, now).await.expect("measure");
    assert_eq!(dashboard.measured_at, now);
    // Nothing ingested: no coverage rows, no quarantine, no lag to report.
    assert!(dashboard.coverage.is_empty());
    assert_eq!(dashboard.quarantine_total, 0);
    assert_eq!(dashboard.lag.fetch_age, None);
    assert_eq!(dashboard.lag.notice_age, None);
    assert_eq!(dashboard.cursor, 0);
    // The canonical counts panel is always populated, at zero.
    assert!(!dashboard.counts.is_empty());
    assert!(dashboard.counts.iter().all(|c| c.value == 0));
}
