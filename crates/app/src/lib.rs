//! The server-side halves of the tender-db binary that are worth testing on
//! their own: the public `/v1` API, the account lifecycle, and the dashboard's
//! measurements.
//!
//! The Dioxus UI and its server functions stay in `main.rs` — they are the
//! binary's own concern, and keeping them out of here means all of this can be
//! booted in an integration test without a browser bundle in sight.

#[cfg(feature = "server")]
pub mod accounts;
#[cfg(feature = "server")]
pub mod admin;
#[cfg(feature = "server")]
pub mod coverage;
#[cfg(feature = "server")]
pub mod snapshot;
#[cfg(feature = "server")]
pub mod supervisor;
#[cfg(feature = "server")]
pub mod v1;
#[cfg(feature = "server")]
pub mod webhooks;
