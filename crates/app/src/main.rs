//! tender-db — an API and browser for public tenders.
//!
//! Dioxus 0.7 **fullstack** app: one crate → a `web` client (WASM) and a `server`
//! (Axum) that hosts the server functions and the Turso database.
//!
//! ```sh
//! dx serve  --platform web        # dev server, hot reload
//! dx bundle --platform web -r     # production bundle (server binary + public/)
//! ```
//!
//! Workspace map (see the root Cargo.toml): `model` holds the shared domain
//! types, `store` the server-only Turso persistence; this crate adds [`api`]
//! (the server-function boundary) and the UI.

use dioxus::prelude::*;

mod api;
mod ui;

const MAIN_CSS: Asset = asset!("/assets/main.css");

/// Web (wasm) entry: hydrate the client.
#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

/// How many reader connections serve the public API. Turso readers parallelise
/// under WAL, so this is the API's real concurrency; the single writer is
/// untouched by it.
#[cfg(feature = "server")]
const READERS: usize = 8;

/// Server entry: the Dioxus router with the public `/v1` API merged beside it.
///
/// The two routers are independent — layers on ours (rate limiting) do not
/// reach dioxus's routes, and the namespaces cannot collide because server
/// functions live under `/api` and the public API under `/v1`
/// (docs/research/api-layer.md §1).
#[cfg(feature = "server")]
fn main() {
    dioxus::server::serve(|| async {
        let db = store::state().await;
        let api = tender_db::v1::AppState::new(db.clone(), db.readers(READERS)?);

        // The ingestion Supervisor (issue 16): a background task owning the
        // writer for its jobs, so a production load runs in-process with zero
        // downtime (readers keep serving over WAL). It is the ONLY path that
        // touches the production DB — the fetch/process/project CLIs are dev
        // tools for scratch databases. `init` spawns the worker + scheduler once.
        let supervisor = tender_db::supervisor::init(db.clone());

        // The webhook delivery sweeper (issue 08): a background task that pushes
        // signed change batches to registered endpoints, woken by the same
        // change-cursor doorbell the SSE uses.
        tender_db::webhooks::init(db.clone());

        if tender_db::admin::enabled() {
            println!("admin: /admin API enabled (TENDER_ADMIN_SECRET is set)");
        } else {
            println!("admin: /admin API disabled (TENDER_ADMIN_SECRET unset → 404)");
        }

        Ok(dioxus::server::router(App)
            .merge(tender_db::v1::router(api))
            .merge(tender_db::admin::router(supervisor)))
    });
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Route {
    /// The dashboard is the product's face, so it owns `/`.
    #[route("/")]
    DashboardPage {},
    #[route("/account")]
    AccountPage {},
    #[route("/tenders")]
    Tenders {},
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}

use ui::{AccountPage, DashboardPage};

#[component]
fn Tenders() -> Element {
    let tenders = use_server_future(api::list_tenders)?;

    rsx! {
        main {
            h1 { "Tenders" }
            match &*tenders.read() {
                Some(Ok(list)) if list.is_empty() => rsx! { p { "No tenders yet." } },
                Some(Ok(list)) => rsx! {
                    ul {
                        for t in list.clone() {
                            li { key: "{t.id}", "{t.title}" }
                        }
                    }
                },
                Some(Err(e)) => rsx! { p { "Failed to load tenders: {e}" } },
                None => rsx! { p { "Loading…" } },
            }
        }
    }
}
