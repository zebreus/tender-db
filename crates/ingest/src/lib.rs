//! Ingestion: fetchers, era/profile parsers, canonical projection.
//!
//! Fetching and processing are separate stages (CONTEXT.md): [`fetch`] only
//! downloads and registers raw packages; parsing/projection never trigger
//! downloads.

pub mod fetch;
pub mod ted;
