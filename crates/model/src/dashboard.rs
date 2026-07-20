//! What the dashboard shows — the shapes the server measures and the wasm
//! client renders.
//!
//! These are report types, not domain types: every number is already resolved
//! server-side (coverage ratios computed, instants turned into ages) so the
//! client is a renderer and nothing more.

use serde::{Deserialize, Serialize};

/// One `/` page load's worth of numbers, fetched as a unit so the panels are
/// always a consistent snapshot of one instant rather than four races.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dashboard {
    /// When the server measured this, unix seconds.
    pub measured_at: i64,
    pub coverage: Vec<Coverage>,
    pub quarantine_total: i64,
    pub quarantine_by_reason: Vec<Count>,
    pub quarantine_recent: Vec<Quarantined>,
    pub lag: Lag,
    pub counts: Vec<Count>,
    /// Award-chaining health per era: how many award Tenders are a lone notice
    /// that never linked to its contract notice (docs/research/ted-legacy-mapping.md §3).
    pub award_linkage: Vec<AwardLinkage>,
    /// The change cursor — the spine everything live hangs off.
    pub cursor: i64,
    /// The git revision the running server was built from (`dev` for a plain
    /// `cargo build`). Measured server-side so the page always shows the rev that
    /// actually served it.
    pub service_rev: String,
}

/// One era's award-chaining coverage. Legacy Tenders chain by transitive OJS
/// references, so a missed link leaves an award stranded as a single-notice
/// Tender — a measurable data-quality gap the research predicts at ~17% for the
/// R2.0.9 era.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AwardLinkage {
    /// The era, i.e. the mapping profile (ted-export-r209, text, eforms:…).
    pub era: String,
    /// Tenders that carry an award decision.
    pub awards: i64,
    /// Of those, the ones with a single notice — an unchained award.
    pub unchained: i64,
    /// `unchained / awards`, 0.0–1.0.
    pub ratio: f64,
}

/// A labelled number; the shape of every count panel row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Count {
    pub label: String,
    pub value: i64,
}

/// Notices held for one (source, profile, year) against what that year is known
/// to have published.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Coverage {
    pub source: String,
    pub profile: String,
    pub year: String,
    /// Notices we hold.
    pub held: i64,
    /// Notices the year published, per the vendored ground truth — `None` where
    /// no ground truth exists (any source but TED, or a year outside the
    /// measured range), in which case `held` stands alone with no ratio.
    pub published: Option<i64>,
    /// `held / published`, 0.0–1.0+. `None` exactly when `published` is.
    pub ratio: Option<f64>,
    /// The ground-truth year is incomplete (the current year), so a ratio below
    /// 1.0 is expected and not a gap.
    pub partial: bool,
}

/// How stale the two stages of the import pipeline are. Both are ages in
/// seconds at `measured_at`; `None` means the stage has never run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lag {
    /// Age of the newest fetched package.
    pub fetch_age: Option<i64>,
    /// Age of the newest ingested Notice.
    pub notice_age: Option<i64>,
}

/// One quarantined payload (ADR-0004) as the drill-down lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quarantined {
    pub reason: String,
    pub profile: Option<String>,
    pub member_path: String,
    pub detail: Option<String>,
    pub first_seen: i64,
}
