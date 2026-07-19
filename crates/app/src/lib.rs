//! The server-side halves of the tender-db binary that are worth testing on
//! their own: today, the public `/v1` API.
//!
//! The Dioxus UI and its server functions stay in `main.rs` — they are the
//! binary's own concern, and keeping them out of here means the API can be
//! booted in an integration test without a browser bundle in sight.

#[cfg(feature = "server")]
pub mod v1;
