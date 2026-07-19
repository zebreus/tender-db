//! Shared domain types — the vocabulary of the client/server boundary.
//!
//! Compiled for both the wasm client and the native server, so: serde types
//! only, no persistence or IO. See CONTEXT.md for the ubiquitous language.

pub mod account;
pub mod dashboard;
pub mod ingestion;

pub use account::{Account, NewToken, Token};
pub use dashboard::Dashboard;
pub use ingestion::Ingestion;

use serde::{Deserialize, Serialize};

/// A public procurement opportunity (never an offer — that's a Bid).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tender {
    pub id: i64,
    pub title: String,
}
