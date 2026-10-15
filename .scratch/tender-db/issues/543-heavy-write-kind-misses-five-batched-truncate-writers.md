# 543 — `heavy_write_kind` leaves out five batched TRUNCATE writers, and the job-control lists are opt-in

Status: ready-for-agent — unit 3 DEPLOYED `014ea47` 2026-10-15 ~18:1x UTC: `reprocess` (between quarantine families and packages), `refold-fields` (between sweep windows and after the sweep) and `rederive-eur` (between windows; a fresh run starts over, only the recovered job resumes) are stoppable. Built `0209343`; its review found a BLOCKER, a per-record stop that stranded a whole-file or container family, fixed in `ee603e0`; the re-review was clean. Open nits (tests only, plus one resume source): pin the held-container override in `reclaim_family` with a mixed container test; pin rederive-eur's per-window job cursor; resume a recovered rederive-eur from its own cursor instead of the global watermark; a stop inside a monthly waits for the whole inner daily (latency only).
  - reprocess stops before every package, and between quarantine families inside one, so a whole-file or whole-container row's records finish together;
  - refold-fields stops between sweep windows and before its first write, and a stopped run writes nothing;
  - rederive-eur stops before every window; a stop clears the watermark and a fresh enqueue starts at the first tender (only a recovered job resumes).

  Next: gate, then deploy.
Was: ready-for-agent — filed 2026-10-15 from the 542 audit (an Explore survey of every non-stoppable kind, then
four adversarial verifiers). Unit 1 built 2026-10-15 01:0x UTC (gate, then main; it deploys with 534 and 542). Unit 2
built 2026-10-15 in the working tree (focused tests green; not gated or committed yet). Next: gate and commit unit 2, then
unit 3.
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

## Unit 1 (built 2026-10-15)

Before editing, every kind still off the list was classified: 27 kinds in five groups, each verdict re-traced by an
independent skeptic, 27 of 27 agreeing. Exactly the five above are batched TRUNCATE writers.
- Writers with one commit, so not batched: `apply-case-reviews`, `unapply-case-reviews`, `apply-country-verdicts`,
  `apply-rehoming`, `restore-dropped-satellites`, `repair-country-typos`, `repair-label-prefixes`,
  `repair-member-twins`, `repair-minted-countries`, `repair-renormalised-identifiers`, `drop-orphan-satellites`,
  `fold-org-countries`, `fetch`.
- Multi-commit writers with no TRUNCATE: `backfill-merged-identifiers`, `fold-provisional-echoes`, `audit-fts-ids`
  (ledger rows), `analyze` (statistics), `rehash-probe`, `probe`. They do not meet the criterion.
- Read-only apart from their report: `disk-census`, `data-quality`, `registry-contiguity`.
- The `-census`, `-packet` and `-probe` report kinds were not re-read one by one; the issue-252 comments describe them
  as storing only a report. Unit 2's contract test makes that a checked claim.

The five are added to `heavy_write_kind` with a comment line each, and to
`heavy_write_in_progress_tracks_the_running_job_kind`.

## Unit 2 — built (2026-10-15, working tree)

Both job-control lists are now exhaustive, and a test enforces it. 91 distinct kinds are pushed: 89 literals at the
`push`/`push_front` sites, plus the two the issues 510/511 arm pushes through its `kind` variable.
- `LIGHT_KINDS` (supervisor.rs, after `heavy_write_kind`) has 51 entries, one reason each. With the belt's 40 that
  makes 91. The 22 unit 1 verdicts are quoted as recorded. The 29 kinds unit 1 did not re-read were each traced:
  every `-census`/`-packet`/`-probe` report kind, `satellite-orphans`, `org-merge-health`, `org-head-drift`, `refresh-org-names` (dry
  only), `case-review-backlog`, `procedure-key-census`, `buyer-role-census` and `reveal-recheck`. Every store fn on
  those paths borrows `Db::reader`, and the only write is `put_report`. `register-archive` writes one registry row
  per unregistered package and TRUNCATEs once after the walk, not per batch. `clear-rebuild-flag` is one statement.
  **No further batched TRUNCATE writer was found**, so `heavy_write_kind` is unchanged.
- `UNSTOPPABLE_KINDS` (after `STOPPABLE_KINDS`) has 37 entries. With `STOPPABLE_KINDS`' 54 that makes 91. The groups:
  - 24 short: one commit, one statement, one download, or seconds of work;
  - 10 deliberate or one-off, with reasons from this issue's audit and the arms' own comments (`refold-denied-schemes`,
    `disk-census`);
  - 3 marked for unit 3: `rederive-eur`, `reprocess`, `refold-fields`.

  `fetch` and `probe` are listed, and their FTS walks stay stoppable by params.
- Tests (in `mod tests`):
  - `pushed_kinds()` reads the production half of `include_str!("supervisor.rs")`. It requires every non-literal
    push to be listed in `VARIABLE_KIND_PUSHES` and checks those kinds against the variable's `let`. It also pins
    `push`/`push_front` as the only callers of `enqueue`.
  - `every_pushed_kind_is_heavy_or_light` fails on a kind in neither list or in both, a stale or duplicate
    `LIGHT_KINDS` entry, a missing reason, or a `heavy_write_kind` arm (read from its body) that nothing pushes.
  - `every_pushed_kind_is_stoppable_or_not` checks the same for the stop lists, checks both lists against
    `stoppable()`, and pins the FTS params case to exactly `fetch` and `probe` under all three FTS names.
  - Red-first by mutation:
    - dropping `xb-packet` from `LIGHT_KINDS` and `fetch-rates` from `UNSTOPPABLE_KINDS` failed both tests, each
      naming its kind;
    - a push through an unknown variable failed both tests too.
- `docs/operations.md`'s cancel section names `UNSTOPPABLE_KINDS` and its test.
