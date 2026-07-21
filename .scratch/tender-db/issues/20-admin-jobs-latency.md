# 20 — read-only queries queue behind the writer under heavy ingestion

Status: needs-verification (second fix landed on main; awaiting prod perf check)

## 2026-07-21 ~11:00 — verification FAILED in prod; second root cause (team lead)

da2ab87 deployed 09:56 UTC. /admin/jobs is now instant (0.4ms, 403-path) —
the mutex half is fixed. But `/` STILL times out, and worse: on da2ab87
every `/` request leaves a thread at 100% CPU essentially forever (observed
threads spinning 49 min / 9 min / 15 s — one per dashboard request; three
of four cores burned on an IDLE queue). Restarted the service 10:56 to
clear them.

Second root cause: the coverage query itself is pathological at scale.
`notice_counts_by_profile_year` joins `notices` (3.5M rows) to `fetches`
with **no index on notices(fetch_id)** — a nested-loop join, O(notices ×
fetches) ≈ billions of row visits, i.e. hours per query. Pre-fix, the
writer mutex serialized these (the "hang"); post-fix the reader pool lets
each request burn its own core instead. The public dashboard is thus an
accidental DoS: every page visit pins a core for hours. The other
coverage reads (import_lag, canonical_counts, award_linkage) need the
same audit.

Fix direction: single-pass aggregation (GROUP BY n.fetch_id over one
notices scan, then join the tiny fetches table) + index on
notices(fetch_id) as an additive migration; plus a short in-app TTL cache
for the coverage panel so dashboard polling doesn't rescan 3.5M rows every
few seconds. Regression test must use a dataset large enough to expose
join order, and assert wall-clock bounds.

Acceptance (unchanged in spirit): `/` and /admin/jobs p99 < 1s under
heavy ingestion, measured in prod, AND no unbounded-CPU queries reachable
from unauthenticated routes.

Observed during the historical backfill (run-driver, 2026-07-21): GET
/admin/jobs occasionally takes ~23s while a process job writes heavily,
then returns instant. **Severity widened during investigation** (team lead,
reproduced on the box): the public dashboard root `/` times out past 30s
under the same load, while /health (32ms) and /v1/tenders (21ms) stay
instant. So this is a public-page outage under ingestion load, not a
cosmetic panel delay.

Acceptance (widened): `/` and /admin/jobs both fast (p99 < 1s) while a
process/project job writes heavily; no behaviour change otherwise.

## Root cause

Every read-only query on `Db` ran on `self.conn()` — the **single writer
connection behind a `Mutex`**. An ingestion job holds that writer for the
length of each transaction (the projection commits in 512-tender
`BEGIN IMMEDIATE … COMMIT` batches — issue 19), so *any* read that routed
through `Db` queued behind the batch and only returned once it committed:
the ~23s stall on /admin/jobs and the 30s+ hang on `/`. `/health` and
`/v1/*` were unaffected because they never touch the writer — /v1 already
reads through the WAL reader pool ("the API's fan-out never queues behind
ingestion"), as do SSE and the webhook change-log feed. The dashboard,
admin, auth, and webhook-management read paths were the exception.

## Fix (2026-07-21) — systemic

`Db` now owns an internal WAL reader pool (`read_pool`, `READ_POOL = 8`)
and a private `reader()` accessor. Every read-only `Db` accessor switched
from `self.conn()` (writer) to `self.reader()` — one line each, no caller
or signature changes. The writer connection is now reserved for writes and
schema only. WAL readers see the last committed snapshot and run in
parallel with the writer, so no read waits on an ingestion transaction.

Swept all 50 `conn()` sites in the store and routed the 26 genuinely
read-only ones (SELECT/read-PRAGMA only) to the reader pool:
- lib.rs: list_tenders, latest_fetch, latest_fetch_period_max,
  current_packages, notice_counts_by_profile(_year),
  quarantine_counts_by_reason, recent_quarantine, import_lag, latest_cursor
- canonical.rs: parsed_notices, parsed_notice, parsed_chunk, award_linkage,
  canonical_counts, scalar, changes_since
- accounts.rs: user, user_credentials, session_user, list_tokens
- webhooks.rs: list_webhooks, webhook, due_webhooks,
  recent_webhook_deliveries
- jobs.rs: recent_job_runs

Left on the writer (writes, or mixed read+write — routing them to a reader
would be a bug): all record_/insert_/create_/delete_/update_/set_ methods,
`clear_canonical`, `resolve_mentions`, `apply_tenders`,
`retire_absorbed_legacy_tenders`, and three traps that read like getters —
`authenticate_token` (SELECT then UPDATEs `last_used_at`, on every API
request), `set_foreign_keys` (state-changing pragma that must land on the
writer the projection uses), and the create_*/delete_webhook methods
(SELECT-then-write). Note: `authenticate_token`'s `last_used_at` touch is a
genuine write and stays on the writer; if that write ever contends under
load it is a separate issue (out of scope here).

Dashboard `/` (`coverage::measure`) uses seven of the routed reads plus
`recent_job_runs`; all are now off the writer.

## Tests

- `store::jobs::reads_do_not_block_on_a_held_writer` (new regression):
  holds the writer in an open `BEGIN IMMEDIATE` transaction and asserts a
  `Db` read accessor still returns (< 1s). Under the old writer-routed read
  this deadlocks against the held guard — the acute form of the stall.
- `job_log_round_trips_newest_first` unchanged in intent (now exercises the
  reader path via the `Db` method).
- Full suites green: `store` (14), `ingest` incl. `project` (20 — projection
  reads parsed data through the pool while writing canonical on the writer),
  `tender-db --features server` (admin end-to-end, accounts, sql incl.
  `sql_execution_does_not_starve_the_api`, webhooks). clippy clean.

## Needs production verification

Confirm `/` and /admin/jobs p99 < 1s during the backfill's heavy
process/project phases. Deploy only at a safe boundary — a restart wipes
the in-memory supervisor queue.

## 2026-07-21 ~11:00 — authed-path timings under ingestion load (run-driver)

Supporting data for the mutex-half fix, measured on the box against da2ab87
while job 1 (`process ted monthly`) was actively ingesting (load avg ~5.3).
Only `/health` and the authed `/admin/jobs` were hit — `/` and
`/api/dashboard` deliberately NOT curled (each da2ab87 dashboard hit pins a
core for hours; see above):

- `/admin/jobs` (X-Admin-Secret): 0.7ms, 4.1ms, 3.6ms, 2.9ms, 2.9ms — all
  sub-5ms, well under the 1s target.
- `/health`: 62ms, 83ms, 111ms.

So the reader-pool fix holds for the authed/JSON paths under real load. The
outstanding failure is confined to the coverage query behind `/` (the missing
`notices(fetch_id)` index) — final acceptance + status flip left to the team
lead once that fix deploys.

## Second fix (2026-07-21) — coverage query + TTL cache

Landed on main atop issue 23 (b499d5a); cherry-picked from an isolated
worktree while 23 re-integrated its snapshot hooks.

- **notice_counts_by_profile_year rewritten join-free**: aggregate `notices`
  by its own columns in one scan (`GROUP BY fetch_id, profile` → thousands of
  rows), then fold up against the small `fetches` table in process. No join
  for the planner to drive from `fetches`, so it is O(notices) irrespective of
  index/table sizes — the O(notices × fetches) nested loop is gone.
- **Index `notices(fetch_id)`** via idempotent SCHEMA `CREATE INDEX IF NOT
  EXISTS` (indexes aren't in the ALTER-only MIGRATIONS list). Honest scope: the
  rewrite is the fix; this index does *not* materially speed the rewritten
  single scan — it is FK-hygiene insurance for any fetch_id lookup. A covering
  `(fetch_id, profile)` index could make the GROUP BY index-ordered but isn't
  warranted given the TTL cache + sub-second single scan. **Deploy note**: the
  first open on the 3.5M-row prod table builds it once (~tens of seconds at
  open) — the 120s health-check grace (230a952) covers it.
- **30s TTL cache** around `coverage::measure`: dashboard polling collapses to
  one measurement per window instead of rescanning every few seconds. App-side,
  no new dep; recompute runs outside the lock (std Mutex never held across await).
- **Audit** of the other coverage reads: only this query had the O(n×m)
  nested-loop shape. import_lag / canonical_counts / quarantine_counts are
  single-table linear scans; award_linkage is join-heavy but PK/subquery-scan
  driven. All now additionally bounded by the TTL cache.

Test `store::coverage_query_is_single_pass_over_notices`: many-fetch dataset,
correct + ordered counts + a coarse wall-clock bound (turso debug-build insert
speed caps the size, so prod is the definitive perf check). Full store +
`tender-db --features server` suites green on the integrated tree (with issue
23); clippy clean.

Needs verification: `/` and /admin/jobs p99 < 1s under heavy ingestion,
measured in prod, and no unbounded-CPU query reachable unauthenticated.

## 2026-07-21 ~12:20 — coverage-query fix verified on bad8dda (run-driver)

Combined deploy bad8dda landed 12:18:57 UTC (durable queue + coverage-query
fix + /health/deep + snapshots). The pathological `/` query is gone:

- **`curl /` under ingestion load: HTTP 200 in 0.002s** (94KB) — the exact
  request that pinned a core for hours on da2ab87 now returns in 2ms. The
  missing `notices(fetch_id)` index closed the nested-loop join. curl-`/` ban
  lifted.
- **`/health/deep`: HTTP 200 in 0.043s**, all four checks passing (database,
  disk used_fraction 0.442, ingest_freshness, last_job ok).

Reader-pool half (from da2ab87) already recorded above (/admin/jobs sub-5ms).
Both halves now green under real ingestion load. Final acceptance / status flip
left to the team lead.
