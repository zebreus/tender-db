# 543 — `heavy_write_kind` leaves out five batched TRUNCATE writers, and the job-control lists are opt-in

Status: ready-for-agent — filed 2026-10-15 from the 542 audit (an Explore survey of every non-stoppable kind, then
four adversarial verifiers). Next: unit 1.
Kind: operations (WAL safety, job control)
Relates to:
- 281 (the under-inclusion this repeats);
- 53 (the 70 GB WAL balloon the belt exists for);
- 252 and 542 (`STOPPABLE_KINDS`, the same opt-in shape).

## Verify

    grep -n '"repair-notice-instants"\|"repair-version-instants"\|"repair-provisional-name-norm"\|"backfill-original-lang"\|"refold-denied-schemes"' crates/app/src/supervisor.rs

- **done**: all five appear in `heavy_write_kind` and in `heavy_write_in_progress_tracks_the_running_job_kind`'s list.
- **open (2026-10-15)**: none of the five is in `heavy_write_kind` (supervisor.rs ~15947–15985).

## What was found

`heavy_write_kind` is the allowlist behind `Supervisor::heavy_write_in_progress`. While it answers true, the coverage
refresher skips its multi-minute full-table scan (`coverage.rs` ~196), so a writer's per-batch TRUNCATE checkpoint can
reclaim the WAL. The layer-presence observer also only touches instead of observing. Its doc says that
under-inclusion is the actual WAL hazard and over-inclusion costs one stale refresh. These five batched writers
TRUNCATE-checkpoint per batch and are not listed:

| kind | where it checkpoints | measured |
| --- | --- | --- |
| `backfill-original-lang` | per 10k-tender window (sup ~7501) | 70 min (job 632, issue 340, older per-row code) |
| `refold-denied-schemes` | inside `unmark_projected_by_ids` and `stamp_tenders_stale`, per wet stride with hits (canonical.rs ~13611, ~13489) | minutes (estimated) |
| `repair-notice-instants` | per write slice, `BEGIN IMMEDIATE … COMMIT` then `checkpoint_on(Truncate)` (canonical.rs ~32779–32836) | — |
| `repair-version-instants` | per slice (canonical.rs ~33112–33120) | — |
| `repair-provisional-name-norm` | per slice (canonical.rs ~35334–35341) | — |

All five are manual, one-off kinds, so the exposure is when an operator runs one. That is when a coverage scan can pin
a reader snapshot across every checkpoint the job takes. `repair-member-twins` checkpoints once after a single
transaction, so it does not meet the criterion.

The root cause is the list's shape. A new batched writer is light by default, and nothing makes its author decide.
`STOPPABLE_KINDS` has the same opt-in shape. That is intended there (a refused cancel is honest), but 542 showed a
long kind can still land without its checkpoint.

## The rest of the audit (verified, not filed as work)

The survey found 16 non-stoppable kinds that looked long. The verifiers cut that down:
- **No kind is scheduled automatically.** Every one is enqueued only through `POST /admin/jobs`.
- `rederive-eur` takes ~10 min (598–646 s, jobs 1001 and 2099). The "~2 h" figure is how long issue 306's walk took
  to reach its wedge, which is fixed. It already resumes from its own watermark. A stop check is cheap hygiene.
- `reprocess` already writes a resume cursor. Its backlog was 12 rows on 2026-10-01. A stop check is worth adding
  before the next large bucket reclaim: copy `reparse`'s shape, between packages and between members.
- `refold-fields`: the sweep itself is read-only before its `expect` gate, so a stop closure costs little. The
  default 8-table sweep has never been timed (46 min was 2 tables). 534 unit 3 narrows it to `notice_ids` (~9 min
  measured, job 1474). Its 7.5 h hold is the `project` after it, which is stoppable apart from grouping and the
  sharded pre-pass.
- **Not worth a stop check**:
  - `repair-placeholder-orgs`: seconds; job 1615 took under a minute, 2488/2489 ~3 s;
  - `backfill-deadlines`, `backfill-titles`, `backfill-currencies`: one-offs that have already run;
  - `backfill-legacy-adjacency`: has not run since Aug 2026; its "40 min" was one query's tail on pre-228 code;
  - `refold-denied-schemes`: deliberately unstoppable, see its comment.

## Units

1. **Add the five kinds** to `heavy_write_kind` and to the test's list. Each gets one comment line naming where it
   checkpoints.
2. **Make the classification exhaustive.** Add a test that collects every kind string `enqueue_request` and the
   schedulers push. Each must be named in exactly one of `heavy_write_kind` or an explicit `LIGHT_KINDS` list, and
   `LIGHT_KINDS` carries a reason per entry. The next batched writer then fails the test until its author picks one.
   Consider the same treatment for `STOPPABLE_KINDS`: an `UNSTOPPABLE_KINDS` list with a reason per entry (short,
   one statement, deliberate).
3. **Stop checks where they pay** (low priority): `reprocess` (between packages and members), `refold-fields` (a stop
   closure into `notice_ids_carrying_fields` between windows), and `rederive-eur` (before each window; the stop line
   says the next run resumes from the watermark). Each joins `STOPPABLE_KINDS` and its contract test.
