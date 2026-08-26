# 295 — Reviews (REV) and E5 contract-completion as canonical entities

Status: BACKLOG (filed 2026-08-26; deferral recorded in CONTEXT.md + spec non-goals)
Kind: capability (data model widening)
Relates to: 291 (same "before more portals?" class — but unlike language/currency,
this is purely additive and can land any time).

CONTEXT.md (resolved 2026-07-19): "Reviews stay notice-layer-only in v1." Review
bodies/decisions (REV) and eForms E5 contract-completion notices are parsed and
stored at the notice layer but never become canonical entities — no review or
completion appears on a Tender's canonical surface.

When picked up: decide the entity shape (a `reviews` satellite per tender/lot;
completion facts on the results graph), then it is the standard additive playbook
(new tables + projection legs + refold of the carrying eras). Measure the carrying
population first — if E5/REV volume is tiny, a thinner "facts on the version"
representation may beat new entity tables.
