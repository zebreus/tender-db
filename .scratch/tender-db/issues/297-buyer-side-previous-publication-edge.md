# 297 — buyer-side previous-publication references as an identity edge (measure, then decide)

Status: BACKLOG (filed 2026-08-26; ADR-0011 line 85: "Same warrant on its face,
unmeasured, and therefore out of scope")
Kind: identity/grouping (ADR-0011 successor)
Relates to: ADR-0011 / issue 236 (the OPP-090 edge that WAS built).

ADR-0011 built the award→contract-notice reference edge (OPP-090) and explicitly
left the buyer-side previous-publication references unmeasured. The work: a
bounded corpus measurement (how many notices carry such references; how many
would merge tenders that today stay split; sample the would-be merges for false
positives — the 168 false-merge method), then EITHER extend the identity fold
with the new edge class + scoped refold, or record a measured "not worth it".
A missed link splits (recoverable); a wrong merge corrupts — keep 236's
strong-reference bar.

## Verify

    grep -c 'BT-125' crates/ingest/src/project.rs

- **done**: a positive count — the buyer-side previous-publication reference is an identity edge class in the fold (scoped refold recorded here), OR this record's foot carries the measured "not worth it"
- **open**: `0` (read 2026-09-19) — the buyer-side previous-planning reference (BT-125) is unmeasured and unbuilt, as ADR-0011 left it; the only reference edge in the fold is OPP-090 (`PREVIOUS_NOTICE_FIELD`, the award → contract-notice class 236 built)
