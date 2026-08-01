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

### Escape hatch: clearing a stale flag without triggering a rebuild

`Db::clear_plan()` (canonical.rs:1504-1529) is the safe clear. It sets `rebuild_in_progress = 0` AND
retires the plan inside one `BEGIN IMMEDIATE` transaction — written that way precisely so the
dangerous end state ("plan retired but still marked rebuilding", which would make the next restart
re-nuke a fully-built layer) is unreachable at any crash point. The plan is transient scratch
(issue 59), so clearing it when no rebuild needs resuming loses nothing.

It is safe **only when no rebuild genuinely needs resuming** — the layer verified intact (8.1M
tenders) and the job queue idle. In that state a set flag is stale by definition: a clean completion
would have cleared it (project.rs:523 calls `clear_plan()` at the end of a run).

Note the circularity that makes this an explicit step rather than something self-healing: a
projection is what normally clears the flag, but a projection fired *while* the flag is set routes to
salvage and nukes the layer. So the flag cannot be cleared by "just running a projection" — it needs
a direct `clear_plan()`.

No admin op exposes `clear_plan()` today. If the flag is ever found set, add a narrow op rather than
hand-running SQL (no-dev-shortcuts-in-prod); it is a three-line dispatch arm alongside `reindex`.

### The daily scheduler makes this an UNATTENDED risk, not just a pre-flight check

`Supervisor::init` calls `spawn_scheduler()` (supervisor.rs:88), which fires `enqueue_daily` at 09:35
Europe/Berlin **every day**; `enqueue_daily` enqueues a `project rebuild=false` (supervisor.rs:964).

So a stale `rebuild_in_progress` is not only a hazard when we fire the refold — the next daily tick
walks into the same salvage branch on its own and drops the layer, with nobody having run anything.
**Check the flag as soon as the API serves, not just before the refold.**

## Precondition 3 — mappings deployed before marking

If the cohort is marked `projected=0` and any projection runs before the `DE1-*` mappings ship —
including a daily reconciliation tick — it re-folds to empty shells again and marks them
`projected=1`, silently consuming the re-fold with no error. Order: deploy mappings → mark → fold.

Satisfied by shipping the mappings in the same batch as the refold op, i.e. before any mark exists.

**A daily tick between the batch deploy and the refold is harmless.** At that point the cohort is
still `projected=1`, so it is not in the change-set and the daily projection does not touch it; the
queue serialises, so it cannot overlap the reindex or the refold either. The only thing that makes a
daily tick dangerous is a stale `rebuild_in_progress` — see precondition 2.

## Mechanism (as built)

No scoped un-mark existed; `clear_canonical` (canonical.rs:1109-1135) is corpus-wide.

**`crates/store/src/canonical.rs`**

```rust
pub async fn projected_notice_count_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64>
pub async fn unmark_projected_for_profiles(&self, profiles: &[&str]) -> turso::Result<u64>
```

The count is a separate method so the guard can abort **before anything is written** — checking after
the mark would mean the damage is already done. `notices_profile` serves the filter. The update is
batched by id range with a TRUNCATE between (mirroring `clear_canonical`'s issue-63 lesson: a single
cohort-wide UPDATE writes a WAL frame per row), and the walk is bounded to the cohort's own MIN/MAX
id, so a clustered cohort costs a few batches rather than a walk of the whole notices table.

**`crates/app/src/supervisor.rs`** — `Spec::Refold { profiles, expect }` (durable), a `"refold"`
request arm, `profiles`/`expect` on `JobRequest`, and `"refold"` in `heavy_write_in_progress()` (same
issue-53 reasoning as `reindex`).

`"refold"` enqueues **two** jobs — mark, then `project rebuild=false` — the same idiom `reprocess`
already uses, so the fold is an ordinary queued projection rather than something bespoke. If the
guard aborts the mark, that projection simply finds an empty change-set and returns
`Report::default()` (project.rs:717-719): a clean no-op needing no special case.

Guard semantics: with `expect` supplied, a match off by more than ±25% aborts, naming the found count
and the profiles, having written nothing. Fire it as:

```json
{"kind":"refold","profiles":["eforms:eforms-de-1.0","eforms:eforms-de-1.1","eforms:eforms-de-1.2"],"expect":218635}
```

**Test** — `unmark_projected_re_queues_only_the_named_profile_cohort` (store lib): scope is exactly
the named profiles (an sdk-1.7 sibling keeps its watermark), the returned count is what was
re-queued, an unmatched profile counts 0 (the mistyped-string case), the cohort lands in
`unprojected_parsed_notice_ids`, the parse layer is byte-untouched (`parse_state` still `parsed`,
values intact — proving projection-only), and it is idempotent.

## Cost and residual risk

Dominant cost is Phase-2 writes with the tender indexes **live** — `strip_tender_indexes` only runs
`if rebuild`, so every satellite insert pays random-position b-tree maintenance (the issue-62 shape,
bounded to the cohort). This is the "random-seek incremental fold per bucket" the `reclaim_only` note
at project.rs:289-292 warns about; expect materially slower per-notice than a rebuild's sequential
fold. Run it alone, and watch WAL — the path checkpoints only at the end (project.rs:809), relying on
`APPLY_NOTICE_BATCH` turnover in between.

## The re-fold changes GROUPING, not just facts

sdk-vendor's fix aliases `DE1-ContractFolderID → BT-04-notice`, so the cohort **gains a procedure key
it never had**. Before: no BT-04 → each notice falls to the island rule → ~218K single-notice island
Tenders. After: real uuids → they group into multi-notice procedures, and per ADR-0003 a shared uuid
merges a DÖE notice with its TED twin.

The incremental path already covers this — it is not a facts-only re-derivation:

- `touched_existing_tender_ids` (canonical.rs:2881-2911) expands on **two** axes: the cohort's
  current tenders (`caused_by_notice_id IN changed`) **and** `SELECT id FROM tenders WHERE
  procedure_key IN (new_keyed_keys)`, where `new_keyed_keys` are the uuids the notices carry *after*
  the fix. That second axis is what pulls the TED twins in, so the ADR-0003 merge is in scope.
- `notice_ids_for_tenders` then expands to the full notice sets of both groups, and the normal global
  grouping SQL runs over that closed set.
- `retire_regrouped_tenders` (project.rs:790, canonical.rs:3013-3060) re-derives each touched
  tender's group_key — `island:N` for a keyless one — and retires it if the new plan no longer
  contains that key, emitting `removed` events. Its doc comment names this case: *"island→keyed
  upgrade"*.

**Legacy check — closed.** The legacy fallback tests `ident.legacy` only over `changed`, not over the
expansion, so a legacy notice pulled in via `notice_ids_for_tenders` would enter the plan without
tripping it. Unreachable here: legacy keys are *constructed* with an `ojs:` prefix
(canonical.rs:64-66) and a BT-04 folder id is stored raw, so no legacy tender is reachable via
`new_keyed_keys` regardless of folder-id format (confirmed by sdk-vendor).

### Soundness of a cohort-scoped incremental for cross-cohort merges

The property that must hold: **for every Tender the plan touches, the plan contains ALL notices that
would group into it in a full rebuild.** Then `build_plan_groups()` — the same SQL a full run uses —
computes the same grouping. For a keyed group with key `U`:

- DE notices carrying `U` → in `changed` (the marked cohort).
- TED notices carrying `U` → their Tender `T` has `procedure_key = U` (grouping is deterministic by
  key), so `T` is returned by `SELECT id FROM tenders WHERE procedure_key IN (new_keyed_keys)` and
  `notice_ids_for_tenders([T])` returns its full notice set.
- Any other unprojected notice carrying `U` → `changed` is *all* unprojected parsed notices, not just
  the cohort, so it is already included.

There is no fourth category: a notice carrying `U` cannot sit in a Tender keyed other than `U`. So
every keyed group in the plan is complete, island groups trivially so, and **a merge-partner TED
tender cannot be left mis-grouped**.

Qualification on "byte-identical to a rebuild": the grouping and content match, the **surrogate
tender ids do not**. A rebuild renumbers from 1 in fold order; the incremental keeps existing ids for
merge partners and appends fresh ones for new Tenders. Semantics match, the id column does not —
relevant if anyone diffs two snapshots.

### 🚩 Precondition 4 — BT-04 needs a uuid gate before this runs

`procedure_key()` (project.rs:2290) accepts **any non-empty BT-04 string**; the `is_uuid` filter
exists only on the sdk-0.1 path (issue 34: *"a missed link splits, it must never wrongly merge"*).
The `DE1-ContractFolderID → BT-04-notice` alias routes DE folder ids through that ungated path. If
any are portal-local reference numbers rather than uuids, every notice sharing such a string
collapses into one Tender — a wrong merge at 218K scale, in the same run that is already retiring
islands and emitting `removed` events.

Gate the DE-1.x folder id on `is_uuid`, as sdk-0.1 already is. This **cannot block a legitimate
merge**: on TED, BT-04 is a spec-guaranteed uuid (the reason the eForms path is ungated at all), so
any DE notice with a genuine TED twin shares a uuid key with it and passes the gate. All-uuid → the
gate is a no-op; not-all-uuid → it prevents an unrecoverable corruption. Empty downside set.

## Validation

1. All 218,635 back to `projected=1`, zero left at 0.
2. Sample DE-1.1/DE-1.2 tenders render facts (title, description, CPV, NUTS, amounts, lots, buyer
   party) — per issue 85's own criteria — **and are no longer 1-version islands**. The grouping change
   is the deeper proof the fix worked.
3. **The tender count should DROP** — but the magnitude is not predictable from the cohort size
   alone, so predict it first rather than eyeballing it. Post-refold the cohort yields ~2,185
   permanent islands (that many carry no ContractFolderID at all, sdk-vendor measured), plus one
   Tender per *distinct* key with no TED match, plus nothing for keys that merge into existing TED
   tenders:

   > drop ≈ 218,635 − 2,185 − (distinct DE keys with no TED match)

   If every folder id were a distinct uuid with no TED twin, the drop would be ≈ **zero**. So "flat"
   only signals failure *given* that keys populated. Two cheap queries make this predictive, to be
   run before firing: `COUNT(DISTINCT value)` over DE-1.x `DE1-ContractFolderID`, and how many of
   those distinct values already exist as a TED tender's `procedure_key`. Check the post-refold
   number against that prediction, not against intuition.

   Read of the signals: DROP matching the prediction = correct · INCREASE ≈ +218K = the identity
   probe failed to find partners · FLAT with keys populated = regrouping did not happen.

   (Earlier drafts of this note claimed the count must stay flat and that a jump would indicate
   duplication from a missing `tenders_procedure_key`. Both were wrong: the re-fold regroups, and a
   missing index makes the identity probe *slow*, never incorrect — a full scan still finds the row.)
4. The CDC feed carries `removed` events for the retired islands — expect a large cursor jump, also
   normal here.

**Canary for precondition 1 is completion time, not the count.** If the fold shows no visible
progress within minutes, kill it and check the two `tenders` indexes rather than letting it grind
through ~10^12 row reads.
