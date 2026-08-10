//! What the dashboard shows — the shapes the server measures and the wasm
//! client renders.
//!
//! These are report types, not domain types: every number is already resolved
//! server-side (coverage ratios computed, instants turned into ages) so the
//! client is a renderer and nothing more.

use serde::{Deserialize, Serialize};

/// The dashboard, as independently-fillable sections (issue 37). Each section is
/// `None` until the background refresher has measured it at least once, and every
/// section fills on its own — a slow full-table scan (coverage, quarantine) can no
/// longer hold the cheap sections hostage, and the boot transient can no longer
/// render as literal zeros. `None` is "measuring since boot…", never "0".
///
/// The default (all `None`) is exactly the pre-first-fill state, so a fresh boot
/// starts honest with no special-casing.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Dashboard {
    /// Cheap status — cursor, revision, staleness. Lands within a second of boot.
    pub system: Option<System>,
    /// The `Contents` counts of the canonical layer.
    pub counts: Option<Vec<Count>>,
    /// The import pipeline per source — fetched → processed → projected (issue 33).
    pub pipeline: Option<Vec<PipelineStage>>,
    pub coverage: Option<Vec<Coverage>>,
    pub quarantine: Option<Quarantine>,
    /// Award-chaining health per era: how many award Tenders are a lone notice
    /// that never linked to its contract notice (docs/research/ted-legacy-mapping.md §3).
    pub award_linkage: Option<Vec<AwardLinkage>>,
}

/// The cheap system-status section: the numbers that need no full-table scan, so
/// they are the first to land after a restart.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct System {
    /// When this section was measured, unix seconds.
    pub measured_at: i64,
    /// The change cursor — the spine everything live hangs off.
    pub cursor: i64,
    /// The git revision the running server was built from (`dev` for a plain
    /// `cargo build`). Measured server-side so the page always shows the rev that
    /// actually served it.
    pub service_rev: String,
    pub lag: Lag,
}

/// The quarantine section (ADR-0004, issues 30/40): the honest three-way split of
/// the held count, the reason breakdown, a recent sample, and the resolution
/// ledger. One section so the panel is always internally consistent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Quarantine {
    /// Every held member — benign, suspected and actionable together, the raw
    /// ADR-0004 count. No longer the headline (issue 30): it is dominated by two
    /// suspected parser gaps, so on its own it overstates real coverage loss.
    pub total: i64,
    /// Of [`Self::total`], how many are STILL HELD — the only number that
    /// describes the present (issue 137 / #29 criterion 6). Everything below
    /// (`actionable`, `suspected`, `by_reason`) is computed over this, not over
    /// `total`, because a reason whose rows were all reclaimed is not a gap and
    /// must not be presented as one.
    pub outstanding: i64,
    /// Reclaimed: reprocessed from the archive and now in the corpus.
    pub reclaimed: i64,
    /// Skipped by policy: identified as duplicates of a sibling already held,
    /// so deliberately never reclaimed. Distinct from `reclaimed` — counting
    /// these as recovered would claim notices entered the corpus that never did.
    pub skipped: i64,
    /// The honest headline: members identified as notices whose content we could
    /// not represent — confirmed real coverage loss, counted over what is still
    /// held. See [`quarantine_class`].
    pub actionable: i64,
    /// Large buckets that look like real notices lost to a single parser gap,
    /// pending investigate-then-fix — flagged distinctly, neither counted as
    /// confirmed loss nor dismissed as benign (issues 35/36).
    pub suspected: i64,
    /// Still-held counts per reason. NOT all-time: see [`Self::outstanding`].
    pub by_reason: Vec<Count>,
    /// The field codes driving the `unknown-field-code` suspected bucket, biggest
    /// first, counted over STILL-HELD rows only (issue 185) — like `by_reason`, a
    /// code whose rows were all reclaimed is history, not a gap. (All-time, the
    /// bucket was ~entirely the one legacy `OC` code, reclaimed 2026-07-29.)
    pub field_code_gaps: Vec<Count>,
    pub recent: Vec<Quarantined>,
    /// Quarantine categories we have diagnosed and fixed — a persistent audit
    /// trail (issue 40). Curated narrative joined at measure time with live
    /// reclaimed/outstanding counts, so a category driven to zero still tells its
    /// story instead of silently vanishing from the panel.
    pub resolved_categories: Vec<ResolvedCategory>,
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

/// One resolved quarantine category (issue 40): a curated ledger entry — the
/// narrative half, source-controlled in the app — joined with the live counts of
/// how much of the bucket has come back. Persists on the panel at zero
/// outstanding so the decision stays visible and traceable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedCategory {
    /// The category in human terms, e.g. "r208 award @REASON".
    pub category: String,
    /// One sentence: what the gap was and what the fix did.
    pub diagnosis: String,
    /// The issue and revision that fixed it, e.g. "issue 31 · 5858159".
    pub fix: String,
    /// When it was resolved, `YYYY-MM-DD` — a fixed historical date, not an age.
    /// `None` while the category is still OUTSTANDING: named so it can be
    /// investigated, but never presented as finished (issue 84 / #29).
    pub resolved: Option<String>,
    /// Matching payloads reprocessed back into the notice layer (live).
    pub reclaimed: i64,
    /// Matching payloads RE-EXAMINED and correctly not ingested, because the
    /// original is already in the corpus (live) — the 2008 per-language duplicate
    /// siblings, issue 84. Shown rather than hidden: folding these into
    /// `reclaimed` would claim notices entered that never did, and folding them
    /// into `outstanding` is the overstatement this count exists to end.
    pub skipped: i64,
    /// Matching payloads still held, awaiting the archive re-walk (live). Falls
    /// to zero as reprocessing catches up; the row stays either way.
    pub outstanding: i64,
}

/// A labelled number; the shape of every count panel row.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Count {
    pub label: String,
    pub value: i64,
}

/// One source's position in the import pipeline (issue 33): the at-a-glance
/// "which stage are we in" the per-year coverage grid can't show. Each stage is
/// its own natural unit — fetch is packages, the rest are notices/tenders — so
/// this is a status strip, not a same-unit funnel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PipelineStage {
    pub source: String,
    /// Notices the source is known to have published (ground truth) — `None`
    /// where no ground truth exists (any source but TED).
    pub published: Option<i64>,
    /// Packages (periods) in the fetch registry, and the period range they span.
    pub fetched_packages: i64,
    pub fetched_from: Option<String>,
    pub fetched_to: Option<String>,
    /// Whether fetching has caught up to the present — the latest fetched period
    /// is in the current year, so downloading is effectively done and what
    /// remains is processing (resolved server-side against the measure clock).
    pub fetch_complete: bool,
    /// Notices processed out of those packages.
    pub processed_notices: i64,
    /// Tenders projected from those notices.
    pub projected_tenders: i64,
}

/// How a quarantine reason relates to notice coverage (issue 30) — the dashboard
/// splits the headline three ways instead of showing one scary total.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuarantineClass {
    /// A member identified as a notice whose content could not be represented —
    /// confirmed real coverage loss, held whole (ADR-0004).
    Actionable,
    /// A large bucket that looks like real notices lost to one parser gap,
    /// pending investigate-then-fix — not counted as loss yet, not dismissed as
    /// benign.
    SuspectedGap,
    /// The reason itself proves the member was never a distinct notice: a wrong
    /// XML root, no publication id, a corrupt archive entry. Benign by design.
    Benign,
}

/// Classify a quarantine `reason`. Evidence-based (issue 30): nothing is called
/// benign without the reason itself proving non-notice. When this split was
/// introduced (2026-07-21) the two big buckets were `SuspectedGap`, not benign —
/// `unknown-field-code` (then 577k) was ~entirely the legacy `OC` field
/// (1995–98), `unparsable-xml` (then 628k) ~entirely "XML with DTD detected"
/// (2008). Per-year coverage showed those years held ~92% vs TED ground truth,
/// so both buckets were *mostly duplicate representations* of notices already
/// held via another member — bounded gaps under investigate-then-fix, filed as
/// issues 35/36 and since largely reclaimed or marked skipped-by-policy (issues
/// 72/73/84); the counts shown are always the still-held remainder. The
/// classification stays: a reason in these families that is held today is
/// suspected until diagnosed (issues 180–182 own the remaining tails). The small
/// uncertain reasons (`not-utf8`, `unknown-customization`) are flagged too
/// rather than assumed benign.
pub fn quarantine_class(reason: &str) -> QuarantineClass {
    // A corrupt archive entry is never a notice, whatever its trailing detail.
    if reason.starts_with("unreadable zip") {
        return QuarantineClass::Benign;
    }
    match reason {
        "unclaimed-content" | "unrepresentable-value" => QuarantineClass::Actionable,
        "unknown-root" | "missing-publication-id" => QuarantineClass::Benign,
        _ => QuarantineClass::SuspectedGap,
    }
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

#[cfg(test)]
mod tests {
    use super::{QuarantineClass, quarantine_class};

    #[test]
    fn quarantine_reasons_class_by_evidence() {
        use QuarantineClass::{Actionable, Benign, SuspectedGap};
        // Identified as a notice, content unrepresentable — confirmed loss.
        assert_eq!(quarantine_class("unclaimed-content"), Actionable);
        assert_eq!(quarantine_class("unrepresentable-value"), Actionable);
        // Big real-notice-shaped buckets — flagged, not benign (issues 35/36).
        assert_eq!(quarantine_class("unknown-field-code"), SuspectedGap);
        assert_eq!(quarantine_class("unparsable-xml"), SuspectedGap);
        // Small uncertain reasons are flagged too, never assumed benign.
        assert_eq!(quarantine_class("not-utf8"), SuspectedGap);
        assert_eq!(quarantine_class("unknown-customization"), SuspectedGap);
        // Benign only where the reason itself proves non-notice.
        assert_eq!(quarantine_class("unknown-root"), Benign);
        assert_eq!(quarantine_class("missing-publication-id"), Benign);
        assert_eq!(quarantine_class("unreadable zip bundle: invalid Zip archive"), Benign);
    }
}
