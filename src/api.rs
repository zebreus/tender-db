//! Public server functions — the client/server boundary.
//!
//! Each `#[get]` compiles to an Axum handler on the server and a fetch stub on
//! the WASM client.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// A public tender, as served to clients.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tender {
    pub id: i64,
    pub title: String,
}

/// All tenders, newest first.
#[get("/api/tenders")]
pub async fn list_tenders() -> ServerFnResult<Vec<Tender>> {
    let db = crate::db::state().await;
    db.list_tenders().await.map_err(ServerFnError::new)
}
