# 67 — cut the serial fold's per-statement floor (skip-stored_chain + prepare-residual + multi-row batch)

Status: CLOSED 2026-09-03 — all three levers found built in `canonical.rs`; measured on job 612 at 641 tenders/s (the predicted ceiling). Was: proposed (spec — build only if the restart's measured fold rate still shows the apply as the wall)
Kind: performance
Blocked by: — (sits on top of dac9187: issue62 fast chain — prepared-stmt fold + parallel pre-pass)
Design owner: proj-fix
Relates to: 64 (fold write side), 63 (plan_state piggyback — future-rebuild lever, does NOT touch this floor)

## Context

Diagnosis (2026-07-25, running fold): the Phase-2 fold is **apply-bound, single-core,
statement-parse-bound** — ~1.4M `conn.execute`/bucket, CPU in turso's parse/plan/VDBE,
disk near-idle. `ad9da49` (prepared statements) removes the PARSE cost (~2.87x on the
uniform inserts). This issue attacks what remains after that: the residual per-STATEMENT
overhead (async dispatch + VDBE setup/step + bind, per call) and a rebuild-only wasted
SELECT. The apply is serial by necessity — `apply_tenders` assigns surrogate ids in
global fold order through the single writer (byte-identity, ADR-0001) — so the goal is
to lower the per-row statement cost, not to parallelize.

Three stacked levers, cheapest first. All must stay BYTE-IDENTICAL: gate on
`project_fold_source` (ParsedFold vs Buckets, all tables + changes + surrogate ids equal)
+ `project_golden` (cross-commit) + `project_resume`.

## (a) Skip stored_chain on rebuild  — trivial, ~2% of statements

`apply_tender_tx` unconditionally runs
`SELECT caused_by_notice_id FROM tender_versions WHERE tender_id=? ORDER BY seq`
(`stored_chain`). On a **rebuild**, `tender_identity`'s rebuild fast-path ALWAYS inserts a
fresh tender id, so that id's `tender_versions` is definitionally empty → the query
parses+plans+probes and returns nothing, 6.96M times.

Fix: `let stored = if rebuild { Vec::new() } else { self.stored_chain(conn, tender_id).await? };`
Then `keep = 0`, the delete loop is empty, all versions are written — exactly what happens
today with an empty `stored`. Byte-identical (rebuild ⇒ fresh id ⇒ empty chain by
construction). One-line guard.

## (b) Prepare the residual per-tender statements — modest

After `ad9da49` the 15 hot INSERTs are prepared. Still unprepared (rebuilt+re-planned per
call via `conn.query`/`conn.execute` with `format!`):
- the head `UPDATE tenders SET current_seq=?, current_published_at=? WHERE id=?` (1/tender)
- `lot_identity`'s `SELECT id FROM lots WHERE tender_id=? AND lot_key=?` (load-bearing even
  on rebuild — it dedupes a lot across a tender's versions, NOT always empty)
- `result_identity`'s `SELECT id FROM {table} WHERE tender_id=? AND notice_id=? AND
  {key}=?` (dedupes a result across a round's re-publication; `{table}`/`{key}` are one of
  3 fixed shapes → prepare 3 statements, not per-call format!)

NOTE: `last_insert_rowid` is already `conn.last_insert_rowid()` in-memory (issue 19) — NOT
SQL, nothing to prepare there. Add these ~5 handles to `TenderInserts`. Low volume vs
facts, so a small win — include because it's free once the struct exists.

## (c) Multi-row INSERT batching of the leaf satellites — the floor-raiser

This is the real lever: collapse per-row leaf INSERTs into
`INSERT INTO t(cols) VALUES (r1),(r2),…`, cutting the statement COUNT ~N-fold (fewer VDBE
invocations / async round-trips), not just the per-call cost.

### Byte-identity — why it's safe for exactly these tables (the linchpin)

The batchable tables carry **no surrogate id that is referenced or emitted**, so their
row-arrival order is unobservable:
- `tender_version_texts / _amounts / _dates / _classifications / _parties` and
  `tender_version_lots` are plain STRICT tables keyed by content columns — their implicit
  rowid is never joined on and never appears in `snapshot()` (every digest ORDERs BY the
  content columns). Insert order → zero effect on output. Batching is trivially identical.
- The result LEAF rows (`tender_version_lot_results`, `_result_winners`, `_result_stats`,
  `_bids`, `_bid_parties`, `_contracts`) likewise carry no own emitted/referenced id — they
  reference the PARENT identity id (`lot_result_id`/`bid_id`), which is assigned BEFORE
  them and unchanged by batching the leaves.
- `tender_versions` (natural PK `tender_id,seq`, no autoincrement) is batchable too.

**Do NOT batch** (surrogate id assigned-then-used within the same version, and emitted):
`tenders`, `lots`, and the `lot_results/bids/contracts` IDENTITY rows. These stay per-row
with immediate `last_insert_rowid` — `lot_id`/`lot_result_id`/`bid_id` flow into the leaf
rows and DO appear in the snapshot, so their sequential assignment must be preserved. They
already require id-then-use, so they can't batch anyway. No risk.

**`changes` — leave per-row initially.** `changes.cursor` is AUTOINCREMENT and IS observed
(snapshot ORDERs BY cursor). A multi-row batch WOULD stay identical IF the VALUES list
preserves the exact `append_version_changes` emission order — but it's the one batchable
table where order is observed, and it's lower-volume than facts. Keep it per-row for a
clean byte-identity story; revisit only if profiling shows it material.

### Shape

Accumulate per-table `Vec<row-tuple>` for the duration of ONE `WRITE_BATCH` transaction
(512 tenders — the buffer is bounded, ~512×~30 facts ≈ ~15k rows, trivial RAM), then flush
chunked multi-row INSERTs just before `COMMIT`. The leaf/fact tables are never read back
mid-write (no SELECT touches them), so deferring their inserts to end-of-batch is safe; the
identity tables (`lots`/`lot_results`/…) keep inserting immediately so their dedup SELECTs
still see prior rows. **Chunk each multi-row INSERT to stay under turso's bind-parameter
limit** (measure it; SQLite is 999 or 32766 — be conservative, e.g. cap
`rows × cols ≤ 900` per statement, so texts@6cols ≈ 150 rows/stmt). A version rarely
exceeds that for one fact type, but per-BATCH accumulation will, so the chunk guard is
required.

## Scope recommendation (proj-fix)

- **(a)+(b): always, ship together.** Near-zero risk, ~1.1–1.3x combined, one small
  commit. Do these regardless.
- **(c): worth the complexity — it is the only byte-identity-preserving lever that lowers
  the serial-writer floor** (the alternative, parallelizing the fold, breaks ordered
  surrogate-id assignment — far more complex and risky). Estimate ~1.5–2.5x ADDITIONAL on
  top of prepared statements. BUT it is the higher-risk piece (transaction restructuring +
  the changes-order caveat), so **stage it second and gate hard**. Build it only if the
  restart's measured fold rate proves the apply is still the wall (single-digit-hours not
  good enough); don't build speculatively.

### Honest ceiling

Even with all three, the fold has a hard floor: ~350M leaf rows inserted into b-trees
through ONE writer. Batching approaches that floor; it can't beat it. Realistic best case
for the fold pass is **low-single-digit hours**, not minutes. The pre-pass is where the big
parallel win already lives (issue 66). The only way to make a FUTURE rebuild's Phase-2
dramatically lighter is issue 63 (plan_state piggyback removes the re-decode) — but that
does not touch THIS apply floor. Set expectations accordingly.

## Validation (byte-identical gate — non-negotiable)

`project_fold_source` (add a bucket/serial variant that exercises multi-row flush) +
`project_golden` (cross-commit) + `project_resume`, all green, before any deploy. The
changes digest (ORDER BY cursor) is the sensitive one — if (c) ever touches `changes`, a
divergence surfaces there first.

## 2026-09-03 — closed by inspection and measurement: all three levers are in the code, and the apply sits at the honest ceiling

Read from `crates/store/src/canonical.rs` today:

* (a) `apply_tender_tx`: `let stored = if rebuild { Vec::new() } else { self.stored_chain(…) }` — the rebuild guard.
* (b) `TenderInserts` carries `head_update`, `lot_lookup`, `lot_result_lookup`, `bid_lookup`, `contract_lookup` as prepared statements beside the hot inserts.
* (c) `flush_rows` issues chunked multi-row `INSERT … VALUES (…),(…)` for `tender_version_lots`, `_texts`, `_amounts` and the other leaf tables; identity rows stay per-row.

(The repository's history is flattened at b4a18a2, so the landing commits cannot be
dated; the code is what counts.)

Measured with them, on job 612 (the 304 campaign's fold, 2026-09-03, epoch-stamped
so `stored_chain` was load-bearing): **apply 7,922,692 tenders / 7,292,947 versions in
12,352 s = 641 tenders/s, 590 versions/s** — 3 h 26 min, 59% of the 5.8-hour fold.
That is the "low-single-digit hours" this issue's honest-ceiling paragraph predicted for
one serial writer; the rate fell 575 → 261 tenders/s across the run as versions per
tender rose 1.0 → 2.1 (issue 96 carries the observation). Nothing left to build here;
further apply speed means parallel writers, which ADR-0001's ordered surrogate ids
forbid. CLOSED.
