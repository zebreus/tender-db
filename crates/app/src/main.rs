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
        let api = tender_db::v1::AppState::new(db.readers(READERS)?, db.cursor_watch());
        Ok(dioxus::server::router(App).merge(tender_db::v1::router(api)))
    });
}

#[derive(Clone, Routable, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
enum Route {
    #[route("/")]
    Tenders {},
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn Tenders() -> Element {
    let tenders = use_server_future(api::list_tenders)?;

    rsx! {
        main {
            h1 { "tender-db" }
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
