//! Ingestion: fetchers, era/profile parsers, canonical projection.
//!
//! Fetching and processing are separate stages (CONTEXT.md): [`fetch`] only
//! downloads and registers raw packages; [`process`] walks what is already in
//! the archive and never downloads.

pub mod eforms;
pub mod fetch;
pub mod package;
pub mod process;
pub mod profile;
pub mod ted;

use sha2::{Digest, Sha256};

/// Lowercase hex sha256 — the content half of Notice identity, and the
/// fetcher's idempotency key.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
