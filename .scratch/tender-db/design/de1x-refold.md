# DE-1.x cohort re-fold — projection-only, scoped (issue 85)

Design for re-projecting the ~218,635 eForms-DE 1.x notices after the `DE1-*` canonical mappings
land. Verdict: **mark the cohort `projected=0` → incremental `project rebuild=false` is correct and
bounded** — but it has three hard preconditions, two of which are safety interlocks.

## Why projection-only is enough

`canonical_name()` (project.rs:2236) maps a **parsed-layer** field id to a canonical name, matching
full id then `stem()`. The `DE1-*` ids are already stored correctly in the value layer; only the
projection's reading of them was wrong. So: no re-parse, no re-fetch, no archive access. Clearing the
watermark and re-folding re-derives the cohort from data already on disk.

## Why it stays bounded (not corpus-wide)

**The legacy fallback does not fire.** project.rs:731-737 aborts the incremental path to a
*full-corpus* projection if ANY notice in the delta is legacy. `is_legacy_profile()` (project.rs:2272)
is `profile == "text" || profile.starts_with("ted-export")`; DE profiles are `eforms:eforms-de-1.x`,
so the fallback is not reachable for this cohort. This was the main unbounded-blowup risk.

**Scoping is genuine** (project.rs:671-681, 788-794): change-set = the unprojected notices; plan =
changed ∪ the touched Tenders' full notice sets; retirement scoped to `touched_tenders`. Untouched
Tenders are never read or written.

**RAM is flat.** Phase-1 streams at `INCREMENTAL_CHUNK = 50_000` (the issue-81 fix) → ~5 passes over
the cohort, peak = one chunk's parsed layer. `unprojected_parsed_notice_ids` returns 218K × i64
≈ 1.7 MB; `all_ids` (changed ∪ touched) ≈ 5 MB. No O(corpus) structure on the path.

## Precondition 1 — the deferred `tenders` indexes MUST exist (issue 82/83 reindex first)

The incremental path runs a **per-Tender identity probe**, canonical.rs:2357-2372, gated exactly on
`if !rebuild`:

```sql
SELECT id, source FROM tenders WHERE procedure_key = ?                    -- tenders_procedure_key
SELECT id, source FROM tenders WHERE source = ? AND island_notice_id = ?  -- tenders_island
```

once per Tender. Plus `touched_existing_tender_ids` (canonical.rs:2899-2908) runs
`SELECT id FROM tenders WHERE procedure_key IN (…)` per `IN_CHUNK = 512` batch (~427 batches).

`tenders_procedure_key` and `tenders_island` are exactly the two indexes a rebuild leaves missing
(`build_tender_indexes` only, not in the schema batch). Without them each probe full-scans 8.1M
tenders: ~200K probes × 8.1M ≈ 10^12 row reads — it does not finish. With them, ~200K B-tree seeks.

**The reindex op is a hard prerequisite of this re-fold, not tail cleanup.**

## Precondition 2 — `rebuild_in_progress` MUST be clear

supervisor.rs:687 + 703: `salvage = rebuild_in_progress()` **outranks** the job's rebuild flag.

```rust
let report = if salvage || *rebuild { project::project(&self.db, true).await }
             else { project::project_incremental(&self.db).await };
```

If the durable flag is still set, a `rebuild=false` job routes into `project(&db, true)` →
`reset_tender_layer()` → the 8.1M layer is dropped and re-folded (~15h). The comment at
supervisor.rs:682-686 records this exact livelock having happened. **Verify the flag is clear
immediately before firing.**

## Precondition 3 — mappings deployed before marking

If the cohort is marked `projected=0` and any projection runs before the `DE1-*` mappings ship —
including a daily reconciliation tick — it re-folds to empty shells again and marks them
`projected=1`, silently consuming the re-fold with no error. Order: deploy mappings → mark → fold,
with the daily scheduler accounted for in the window.

## Mechanism

No scoped un-mark exists; `clear_canonical` (canonical.rs:1109-1135) is corpus-wide. Add:

```rust
/// Re-queue a profile cohort for the incremental fold (issue 85): the parsed layer is
/// intact but the projection mis-read it, so clearing the watermark alone re-derives
/// them. Batched by id range with a TRUNCATE between — a single 218K-row UPDATE writes
/// a WAL frame per row (issue 63).
pub async fn unmark_projected_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64>
```

- `notices_profile` index exists, so the profile filter seeks.
- Batch by id range + TRUNCATE checkpoint between, mirroring `clear_canonical`'s issue-63 lesson.
- **Return the affected count**; the caller asserts it is ≈218,635 and aborts on a wild mismatch —
  cheap insurance against a typo'd profile string matching a far larger set, on an operation whose
  failure mode is "silently re-fold the corpus".

Expose as a supervisor op (`{"kind":"refold","profiles":[…]}`) that enqueues mark → `project
rebuild=false`, rather than hand-run SQL (no-dev-shortcuts-in-prod; durable and observable like every
other job). Add `"refold"` to `heavy_write_in_progress()`, same reasoning as `reindex`.

## Cost and residual risk

Dominant cost is Phase-2 writes with the tender indexes **live** — `strip_tender_indexes` only runs
`if rebuild`, so every satellite insert pays random-position b-tree maintenance (the issue-62 shape,
bounded to the cohort). This is the "random-seek incremental fold per bucket" the `reclaim_only` note
at project.rs:289-292 warns about; expect materially slower per-notice than a rebuild's sequential
fold. Run it alone, and watch WAL — the path checkpoints only at the end (project.rs:809), relying on
`APPLY_NOTICE_BATCH` turnover in between.

## Validation

1. All 218,635 back to `projected=1`, zero left at 0.
2. Sample DE-1.1/DE-1.2 tenders render facts (title, description, CPV, NUTS, amounts, lots, buyer
   party) — the point of the exercise, per issue 85's own validation criteria.
3. **Tender count stays ~8,107,362.** A re-fold upserts by natural key and must NOT add ~218K
   tenders. A jump of roughly the cohort size means the identity probe missed and the cohort was
   *duplicated* — precisely what a missing `tenders_procedure_key` produces, so the count is a direct
   canary for precondition 1. Take it immediately before and after.
