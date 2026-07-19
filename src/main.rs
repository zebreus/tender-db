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
//! Module map:
//!   - [`api`] — public server functions (the client/server boundary).
//!   - [`db`]  — server-only Turso persistence.

use dioxus::prelude::*;

mod api;
#[cfg(feature = "server")]
mod db;

const MAIN_CSS: Asset = asset!("/assets/main.css");

/// Web (wasm) entry: hydrate the client.
#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

/// Server entry: the standard Dioxus router.
#[cfg(feature = "server")]
fn main() {
    dioxus::server::serve(|| async { Ok(dioxus::server::router(App)) });
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
