//! Public server functions — the client/server boundary.
//!
//! Each `#[get]` compiles to an Axum handler on the server and a fetch stub on
//! the WASM client.

use dioxus::prelude::*;
use model::Tender;

/// All tenders, newest first.
#[get("/api/tenders")]
pub async fn list_tenders() -> ServerFnResult<Vec<Tender>> {
    let db = store::state().await;
    db.list_tenders().await.map_err(ServerFnError::new)
}
