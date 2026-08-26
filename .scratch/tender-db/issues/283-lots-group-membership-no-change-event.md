# 283 — a lots-group membership change emits no change-feed event

Status: RESOLVED-DEPLOYED 2026-08-26 (owner) — fix + red-first fixture committed
(`fbc974c`), full `ops/check.sh` green (64 suites, golden/equivalence unaffected),
deployed to prod (rev `fbc974c`, /health green, queue idle). Forward-only: past missed
events are not retro-emitted (LOW severity). See "Resolution" below.
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

## Resolution (2026-08-26, owner)

Added `prev.group_members != v.group_members` to the tender-level `changed` gate in
`append_version_changes` (canonical.rs ~4671). A composition-only version now emits a
`tender changed` row, so a `/v1/changes` / SSE subscriber sees it and re-reads. The
fold keeps `group_members` in canonical sorted order (project.rs `sort_unstable` +
`dedup`; silent versions clone the already-sorted prev), so a `Vec` compare equals a
set compare — no spurious change on a byte-identical re-fold.

**Fixture (red-first proven):** `a_group_composition_change_emits_a_tender_changed_event`
(project_incremental.rs) folds three versions of one keyed Tender that are identical in
title, lots and lot-facts, differing ONLY in GLO-1's composition — v1 = {LOT-1,LOT-2},
v2 = {LOT-1,LOT-3}, v3 republishes v2's composition unchanged. Without the fix the change
feed read `[1:added]` — the v2 composition move was invisible. With it, `[1:added,
2:changed]`, and v3 adds nothing (no false positive).

**A trap worth recording** (cost ~5 iterations): the fixture first carried a buyer org,
which was masking the bug. A `Fact::Party { role, organization_id, notice_id, section_id }`
(canonical.rs:907) embeds the `notice_id` it came from, and `notice_id` differs every
version — so a republished party makes `prev.facts != v.facts` fire on EVERY version,
and the composition delta is never the sole change. Isolating a "changed only X" case in
this fold means publishing NO party (and no other notice-provenanced fact) on the versions
under test. The `pub_at`/dispatch date is safe — it is a version field, not a fact.

Scope check: `ops/check.sh` fully green including the golden and full-vs-incremental
equivalence suites, so no existing fixture published a composition-only change — this event
class was genuinely absent, not merely masked. The fix is forward-only: it does not
retro-emit the events missed for past composition changes (LOW severity, narrow consumer).
