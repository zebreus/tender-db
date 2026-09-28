# 444 — past p0 echo folds left the losers' `organization_names` variants pointing at deleted organizations

Status: ready-for-agent — filed 2026-09-28 from issue 442 step 2. Mine to take. Size unknown on prod: measure first.
Kind: data integrity (organization layer), small
Relates to: 442 (step 2 found and fixed the cause), 351 (the p0 fold), 353 (its campaign, ~5.7M provisional rows
deleted with foreign keys off), 443 (the sweep's pre-image log is the restore-shape precedent), 354 (stale satellite
rows tolerated only where a join filters them)

## What is wrong

`fold_provisional_echoes` (rule `p0`) took a mention-only shortcut for a loser with no tender rows. It never moved
the loser's `organization_names` variants, then deleted the loser with foreign keys OFF (issue 352's bracket), so
each variant stayed behind under a dead `org_id`. The code is fixed (442 step 2: the shortcut is gone and the fold
runs FK-on). The rows the earlier wet runs left are still there.

Impact is small but real:
- They are foreign-key violations. A future `PRAGMA foreign_key_check` or integrity audit reports them.
- A variant in another language (a FRA or ENG rendering) is evidence of the keep's name that never reached it. The
  keep never shows that variant, and issue 300's cross-language matcher never reads it.
- `build-org-match-keys` reads `organization_names`, so it writes keys for dead ids. The wall's counts join
  `organizations` (354), so those do not count, but they are dead weight.

Expected size: small. Provisional rows rarely carry variants. 443's sweep counted 588 variants over 1,768,353
provisional orphans, so millions of p0 losers suggest thousands of rows, not millions.

## What to do

1. **Measure** with a bounded, windowed job over `organization_names` in `org_id` order. For each window, count the
   rows whose `org_id` has no `organizations` row, and among those how many resolve through `org_merge_log` (loser →
   keep, `chase_merged_org`) to a standing org. This is a writer-free reader walk. The 78M-rowid table is not a
   bounded `/v1/sql` read, so it has to be a job (dry default, 432 plan shape).
2. **Repair (wet):** for each orphan with a standing chased keep, `INSERT OR IGNORE` the variant under the keep (the
   keep's own variant wins a language collision, the `repoint_org_references` rule), then delete the orphan. Orphans
   with no chase target are deleted into a pre-image log (443's `org_sweep_log` shape, or a sibling table). Plan
   parity as usual.
3. **Verify:** the dry count reads 0 after the wet run.

## Verify

    dry repair job: orphaned variants = 0
