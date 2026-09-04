# 352 — the R2/E0/R3 merge loops pay foreign-key proving on every loser delete (~0.4 s a row)

Status: ready-for-agent (filed 2026-09-04 from issue 351's first wet slice)
Kind: throughput (organization layer merge machinery) — small, measured
Relates to: 351 (where it was measured and fixed for the provisional fold), 19 (the projection's precedent), 300 Stage 2/3 (R2/R3), 329 (E0), the `lib.rs` re-parse note on mention deletes

## Observed

Issue 351's first wet slice folded ~2 rows/s with every read instant and NVMe
idle: the cost is the engine proving on the write path, per `DELETE FROM
organizations`, that no row of the five child tables still references the
parent. `lib.rs` recorded the same for mention deletes (~2.2 s each). The R2
pass on 2026-09-04 (job 638) merged 1,846 groups in 745 s — the same ~0.4 s
per group, for the same reason: `match_org_identifiers_r2` (and R3 through
it) moves every reference off the loser through `repoint_org_references` and
then deletes the loser row with foreign keys on.

## Proposal

Bracket the wet loop of `match_org_identifiers_r2` (R2, E0, and R3's use of
it) with `PRAGMA foreign_keys=OFF` … `ON` on the writer, exactly as
`fold_provisional_echoes` now does (issue 351): the loop is self-consistent by
construction, the pragma is a no-op inside a transaction so it brackets the
loop, and the restore runs whatever the loop returns. Pin it with
`Db::foreign_keys_enabled` before and after in `r2_merge.rs` / `e0_merge.rs`.
Expected: the E0 plan (2,151 groups) from ~15 min to under a minute, and the
329 wet run — when it is allowed — cheap enough to run uncapped.

## Done when

- the bracket is in the R2 loop with the test probe;
- one wet merge on prod (E0 or R2) shows the per-group time an order of
  magnitude down.
