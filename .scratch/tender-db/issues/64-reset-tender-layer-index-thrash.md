# 64 — reset_tender_layer thrashes tender indexes on a resume over a partial layer

Status: open
Kind: performance
Blocked by: —

## Symptom

On the issue-62 cutover (2026-07-25), triggering `rebuild=true` over a partial
tender layer (the ~226k-tender throwaway fallback) spent a long silent phase at
100% disk %util doing ~3900 RANDOM 4KB reads/s before the first `[project]` line.
That is `reset_tender_layer` clearing the old content, but with the random-key
tender indexes still present.

## Root cause

In `project_with_progress_phase2` the order is:

```
if rebuild {
    db.reset_tender_layer().await?;    // DELETEs the 17 content tables …
    db.strip_tender_indexes().await?;  // … but the deferred indexes are dropped AFTER
}
```

`reset_tender_layer` does `DELETE FROM <table>` on the 17 tender-content tables.
When those tables still carry the deferred tender indexes (a resume over a layer
built rebuild=false — e.g. the fallback — leaves them fully indexed), every row
delete maintains the random-key indexes → a random-position b-tree write/read
storm (the same issue-60/62 pathology, here on delete). Cost scales with the
partial layer's satellite row count, so a resume interrupted late (e.g. 50%
Phase-2) could pay a very large one-time delete cost.

## Fix (either, cheap)

1. **Reorder:** call `strip_tender_indexes()` BEFORE `reset_tender_layer()` so the
   DELETEs run index-free (sequential page frees, no random maintenance). Minimal,
   low-risk — a two-line move.
2. **Better — DROP not DELETE:** have `reset_tender_layer` DROP+recreate the 17
   content tables (O(1) on turso, as it already does for `tenders` and as
   turso-internals established for large throwaway clears) instead of per-row
   DELETE. Removes the cost entirely regardless of partial-layer size. Costs a
   little DDL duplication (mirror each CREATE), same pattern already used for
   `tenders`.

Prefer (1) for minimality unless the DDL-duplication of (2) is judged worth the
guaranteed O(1). Both keep the layer byte-identical (they only change HOW the
transient partial rows are cleared before a from-scratch fold).

## Note

Not a correctness issue — the reset completes and the fold is byte-identical. It
is a one-time cost on a resume-over-partial. On a FRESH rebuild the tables are
already empty so it is free; it only bites a resume that inherited a populated,
indexed layer.
