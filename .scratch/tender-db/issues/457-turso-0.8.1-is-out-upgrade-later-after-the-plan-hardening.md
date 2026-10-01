# 457 — turso 0.8.1 is out (2026-09-29): upgrade later, after the plan hardening that 0.7.2 wants anyway

Status: ready-for-agent — EVALUATED 2026-10-01 (workflow `wf_1f19977e-497`: 4 readers over the published 0.7.2 and
0.8.1 crate sources, 13 risky claims re-checked by challengers; report `.scratch/tender-db/457-turso-0.8/evaluation-2026-10-01.md`).
Verdict: upgrade LATER. The durability gain is taken now with a pragma (issue 458). NEXT: unit 1 (R2, below) on 0.7.2.
Kind: dependency / platform
Relates to: 166 (the 0.7.2 bump, the template), 425 (its step 4 waited for "0.8.0 stable"), 438, 239, 329, 421, 428,
429, 442, 445, 458

## Why now

`crates.io` shows `turso` 0.8.0 and 0.8.1 stable, 0.8.1 published 2026-09-29. 425's open step said to re-check at
0.8.0 stable, where "the vendored patch must be re-applied (or dropped) anyway". Nothing on the board tracked the
bump.

## What the evaluation found (details and citations in the report)

- **On-disk format: the same in both directions.** The DB header code and the WAL frame format are byte-identical,
  nothing is migrated at open for a WAL-mode file, and all of tender-db's DDL parses under both versions. A rollback
  stays valid if the bump ships no new ALTER, never enables MVCC, and creates no 0.8.1-only objects. Unverified:
  the live header bytes (a free metadata read; do it before the bump).
- **The biggest gain is durability.** 0.8.1 fsyncs the WAL before a checkpoint backfills it, which 0.7.2 does not
  (issue 458). It is taken NOW on 0.7.2 with `synchronous = FULL`.
- **Issue 438's root is fixed in the engine.** The interrupt and deadline check runs every 256 instructions, not before
  each one. The SDK still hides `interrupt()`, so `crates/vendor/turso` cannot be deleted. It has to be re-vendored
  from the 0.8.1 SDK, with the 28-line pass-through re-appended.
- **Every documented planner trap stays:** 239 (no view pushdown), 329 (IN on a non-leading column), the GROUP BY
  index steal, 323's `+parse_state`, 421's CROSS JOIN, 428/429 (sqlite_stat1 only, organizations excluded), 442 (the
  FK parent probe) and 445 (per-statement FK checks).
- **Predicted regressions (from the source, no EXPLAIN run):**
  - **R2:** correlated `(SELECT MAX(x.seq) FROM tender_versions x WHERE x.tender_id = t.id)` in five `read.rs` page
    queries can be unnested group-first into a GROUP BY over ~14.5 M rows, because `LIMIT ?` (not a literal) does
    not discount the correlated form. Two COUNTs over `organization_mentions` qualify too.
  - **R3:** `IS` / `IS NULL` become index seek keys, so `ORG_MERGE_SCAN_SQL` (canonical.rs ~1248) is predicted to
    flip to `organizations_identifier_id` plus a temp B-tree sort over ~24.6 M rows.
  - **R4:** the `/v1/sql` surface widens silently: recursive CTEs (any self-referencing CTE recurses, keyword or
    not), 10 window functions, custom frames with EXCLUDE, and 3 scalars. `classify()` is a deny-list there, and no
    test pins the old refusals.
  - **R5:** about 40 EQP-text pins need re-baselining ("USING COVERING INDEX" appears on SEARCH lines).
- **Compile break:** one line, `crates/app/src/v1/sql.rs:1229` (`Cmd::ExplainQueryPlan { .. }`).

## Plan (units)

1. **R2 on 0.7.2, worth doing regardless:** replace the correlated MAX subqueries with `t.current_seq`, after
   proving the equivalence (`current_seq` is maintained at lib.rs ~842; a test that the two agree across a
   multi-version fold, including withdrawn and superseded versions). It deletes planner dependence from the hot read
   path.
2. **R3 on 0.7.2:** pin the IS NULL statements (`+identifier IS NULL` or an explicit index choice) with plan tests
   that hold under both engines, plus a result-equivalence test for the IS NULL seeks.
3. **R4 decision:** refuse self-referencing CTEs, and refuse or document the window functions, EXCLUDE frames and
   new scalars. Pin each as a canary in `tests/sql.rs` before the bump, so the bump's gate catches the widening.
4. **Wait for 429's step-0 capture diff** (Sunday 2026-10-04 jobs and snapshot) so the two studies do not confound.
5. **The bump:** a separate worktree, `cargo clean`, re-vendor the SDK, run `ops/check.sh` from clean (~35 min),
   re-baseline the EQP pins one by one, each with a reason.
6. **On-box test on reflink copies, queue idle:** read-only plan and timing diff (0.8.1 `Builder::read_only`), then a
   write phase with a header diff, `integrity_check` on the touched tables, and a 0.7.2 reopen (the rollback proof).
   Never start the app server on a copy.
7. **Deploy:** clean stop, a 0-byte `-wal`, a reflink snapshot as the rollback point, then start. Verify through one
   daily tick and one Sunday cycle. Revisit 458's FULL vs NORMAL with a measurement.

## Verify

    grep -n '^turso = "=' Cargo.toml

- **open** (2026-10-01): `44:turso = "=0.7.2"`.
- **done:** `44:turso = "=0.8.1"` (or later), deployed, through one Sunday cycle.
