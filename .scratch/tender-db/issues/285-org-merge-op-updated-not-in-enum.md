# 285 — org-merge survivor emits op="updated", outside the documented change enum; poll/webhook and SSE disagree

Status: FIXED in working tree (2026-08-26, owner), awaiting gate+deploy
Kind: correctness (change-feed contract)
Severity: LOW
Relates to: 234 (the merge that emits it), 46 (change-feed protocol)
Found by: the 2026-08-26 change-log review; verified.

## The bug

`merge_provisional_organizations_batch` emitted the survivor's change as
`append_change(conn, "organization", keep, None, "updated", now)` (canonical.rs
~4323) — `"updated"` is outside the schema's own enum (the `-- added | changed |
removed` comment) and the public docs contract. `organization` is a public change
kind served on all three transports, but inconsistently: poll (`/v1/changes`) and
webhook pass the raw op straight through, so consumers coded against the documented
enum receive an unknown `"updated"` (dropped or errored); SSE never emits it as-is
(the survivor row has `seq=0` → reclassified to `added`). One merge, three feeds,
three behaviours.

## Fix (shipped)

Normalized the survivor op to the documented `"changed"` at write time (the mention
set genuinely changed). SSE was match-state-driven so it is unaffected; poll and
webhook now emit `changed`, matching the enum and each other. `"updated"` appeared
nowhere else in the tree (grep), so no consumer branched on the literal — safe.
