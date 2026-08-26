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
