//! The quarantine resolution ledger (issue 40): the source-controlled, curated
//! half of the dashboard's "Resolved categories" audit trail.
//!
//! As a quarantine category is diagnosed, fixed and reprocessed, its live count
//! falls to zero and the story would vanish. So the *narrative* — what the
//! category was, how it was fixed, when — lives here as reviewable data, keyed to
//! the quarantine rows it describes. The *counts* stay live: [`crate::coverage`]
//! joins each entry against `Db::quarantine_resolution` at measure time. Adding
//! an entry is part of landing a quarantine fix.

use serde::Deserialize;

/// The vendored ledger, beside this module as data (not code) so adding an entry
/// on a fix is an edit to a table, not a patch.
const LEDGER: &str = include_str!("../data/quarantine-ledger.json");

/// One curated entry: the narrative, plus the key that selects its quarantine
/// rows. `profile` and `detail_like` are optional narrowers — `detail_like` is a
/// SQL `LIKE` pattern pinning a sub-bucket within a reason (e.g. `%@REASON`).
#[derive(Clone, Debug, Deserialize)]
pub struct LedgerEntry {
    pub category: String,
    pub reason: String,
    pub profile: Option<String>,
    pub detail_like: Option<String>,
    pub diagnosis: String,
    pub fix: String,
    pub resolved: String,
}

/// The resolution ledger, newest fixes as ordered in the file. Panics on a
/// malformed file — it is vendored and covered by a test, so a parse failure is a
/// build-time mistake, not a runtime condition to tolerate.
pub fn resolution_ledger() -> Vec<LedgerEntry> {
    serde_json::from_str(LEDGER).expect("quarantine-ledger.json is valid")
}

#[cfg(test)]
mod tests {
    use super::resolution_ledger;
    use model::dashboard::{QuarantineClass, quarantine_class};

    #[test]
    fn the_vendored_ledger_parses_and_seeds_issue_31() {
        let ledger = resolution_ledger();
        // The two issue-31 fixes are seeded and non-empty in every field.
        assert!(ledger.len() >= 2, "issue 31's two fixes are seeded");
        for entry in &ledger {
            assert!(!entry.category.is_empty(), "a ledger entry names its category");
            assert!(!entry.diagnosis.is_empty(), "a ledger entry states its diagnosis");
            assert!(!entry.fix.is_empty(), "a ledger entry cites its fix");
            assert!(!entry.resolved.is_empty(), "a ledger entry dates its resolution");
        }
        // The issue-31 seeds resolved actionable real-notice loss (later entries
        // resolve other classes — e.g. issue 35's OC/ON is a suspected-gap bucket).
        let seeds: Vec<_> = ledger.iter().filter(|e| e.fix.contains("5858159")).collect();
        assert_eq!(seeds.len(), 2, "issue 31's two fixes are seeded");
        assert!(seeds.iter().all(|e| quarantine_class(&e.reason) == QuarantineClass::Actionable));
    }
}
