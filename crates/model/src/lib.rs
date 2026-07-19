//! Shared domain types — the vocabulary of the client/server boundary.
//!
//! Compiled for both the wasm client and the native server, so: serde types
//! only, no persistence or IO. See CONTEXT.md for the ubiquitous language.

use serde::{Deserialize, Serialize};

/// A public procurement opportunity (never an offer — that's a Bid).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tender {
    pub id: i64,
    pub title: String,
}
