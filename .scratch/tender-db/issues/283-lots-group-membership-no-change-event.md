# 283 — a lots-group membership change emits no change-feed event

Status: DIAGNOSED (2026-08-26, owner — exploratory review, verified NEW)
Kind: correctness (change-feed completeness)
Severity: LOW (the feed carries ids/seq only; group membership is a narrow consumer surface)
Relates to: 237 (projects the membership at all — DONE; this is the change-feed diff gap), 164 (missing events class)
Found by: the 2026-08-26 change-log/SSE review.

## The gap

`fold` carries `group_members` forward with per-group supersession and writes them
to `tender_version_lot_group_members` per version (project.rs ~3303; write_version
canonical.rs ~4391). But `append_version_changes` (canonical.rs ~4648) never diffs
`group_members`: the tender-level `changed` gate is
`Some(prev) if prev.facts != v.facts || prev.rounds != v.rounds` (canonical.rs
~4664) — it omits `v.group_members` entirely, and there is no per-membership diff
loop. A version whose ONLY delta is lots-group composition writes new membership
rows but appends zero `changes` rows.

## Failure scenario

A contract notice defines LotsGroup GLO-1 = {LOT-1, LOT-2}; a later notice
republishes GLO-1 as {LOT-1, LOT-3} (or the issue-237 sole-group inference
reassigns members among already-present lots) while republishing identical
title/values/rounds/lot-facts. `fold`'s per-group supersession updates
`tender_version_lot_group_members` for the new seq, but the diff gate sees
`facts==`, `rounds==`, `lots==` and appends nothing, so a `/v1/changes` or SSE
subscriber never learns which lots a group (and thus which lots a group-scoped
bid) now covers.

## Fix direction

Add `prev.group_members != v.group_members` to the tender-level `changed` gate, or
diff group membership per group and emit a `lot`/`tender` `changed` row when
composition changes. Small, byte-identity-neutral (it only adds change rows,
never touches canonical data). Pin with a fixture: a version that changes only
group composition must produce a `changed` row.
