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

## 2026-07-21 ~13:55 — CORRECTION: `/` still stalls ~15-25s under ACTIVE write load (run-driver)

**Supersedes the "2ms, ready to close" note above** — that reading (12:20) was
taken during the dedup fast-forward when job 1 was writing NOTHING (notices=0),
so it never exercised write contention. Re-tested once job 1 was into REAL
parsing (notices climbing), i.e. the genuine heavy-write load the acceptance
asks for. Result is not clean:

Interleaved `curl /` with job state:
- `/` requests fired **while notices were actively growing** (writer committing):
  HANG — timed out at 12s / 30s (http=000). Two independent bursts, 4 hung reqs.
- `/` requests fired **while notices were flat** (writer idle between commits):
  2–13ms, HTTP 200.

Per-thread CPU during the hangs: 3 blocking threads at ~100% simultaneously,
box 0% idle — one core pinned per in-flight `/`. BUT bounded: a 60s watch showed
threads>50%CPU going 1→0→1→1 and idle recovering to 44–73% — the pinned queries
**self-clear in ~15-25s**, they do NOT spin forever. So this is a big improvement
over da2ab87 (hours, permanent pin → the notices(fetch_id) index fixed that), but
it is **not** "p99 < 1s under ingestion": during the backfill's sustained writes,
each `/` hit takes ~15-25s and burns a core for that duration.

Likely mechanism: the coverage aggregation still costs ~15-25s cold; when the
writer is active the page cache is churned so every `/` runs cold, and results
are only fast when the writer pauses (warm/cached). Needs a fix before resolve —
candidates: memoize the coverage result with a short TTL (invalidate lazily, not
per-write) or compute it off the request path. **No lasting harm from the probe:
cores cleared in ~20s, job 1 kept progressing (pkg 220, notices 15.6k→29k).**
Recommend issue 20 stays open. Stopped probing `/` (each hit costs ~20s of a core);
monitoring remains /health + authed /admin/jobs only.

## Third fix (2026-07-21) — coverage measured off the request path

The run-driver re-measure above confirmed the two gaps in the bad8dda TTL
cache: it computed on the REQUEST path, so every cold window paid the scan and
concurrent misses weren't single-flighted; and under sustained write churn the
page cache thrashes so the scan is ALWAYS cold — the TTL never protected.

Fix: move coverage measurement off the request path entirely (issue 20 part 3).
`coverage::init(db)` spawns a background refresher that recomputes on a 60s
interval, keeping the last good snapshot on error. `coverage::latest()` — a
synchronous, store-free read of that memoized snapshot — is what
`/api/dashboard` serves; it takes no `Db` and is not `async`, so "a request
recomputes" is a compile error, not a runtime hope. No request ever triggers a
scan, so public traffic cannot pin a core at all — the availability property we
actually want, stronger than any TTL. First boot serves the empty default ("no
data yet") until the first refresh fills it; the dashboard's live job progress
comes from the supervisor, which never scans.

- coverage.rs: removed the request-path TTL cache; added the background
  refresher (`init`) + memoized snapshot (`latest`); `measure` (the one-pass
  scan) is now public and called only by the refresher (and the honest-empty
  test). main.rs spawns `coverage::init` beside the other background tasks;
  api.rs `/api/dashboard` returns `latest()`.
- Test `latest_serves_the_memoized_snapshot_without_measuring`; the request
  path being sync + Db-free is the compile-time guarantee that it never scans.
  Full app suite (35 lib + accounts/admin/api/sql/webhooks) + wasm check + clippy
  green.

Needs verification: `/` p99 < 1s under sustained ingestion in prod (served from
the snapshot; the 60s background scan stays off the request path).

## 2026-07-21 15:12 — b0a5cdb coverage-refresher: `/` fast, no core-pin (run-driver)

The 20-pt3 background coverage refresher landed in b0a5cdb. Re-tested `/` — the
stall + core-pin I found on bad8dda is gone:

- 10× `curl /` during job 1's activity: **1.0–2.0ms, all HTTP 200.**
- 3× authed `/admin/jobs`: 0.6–1.0ms.
- Immediate per-thread CPU after the burst: **0 threads >50%**, idle ~47%,
  load 2.55 — no core pinned (bad8dda pinned one core per in-flight `/`).
- `/health/deep`: 200 in 43ms.

The refresher decouples `/` from the query: requests read a maintained coverage
snapshot and can never trigger the scan inline, so `/` is flat regardless of
ingestion state. **HONEST CAVEAT:** this burst ran while job 1 was in the dedup
**re-walk** (notices=0 → read-heavy, minimal write commits). My bad8dda stall
correlated specifically with active write **commits** (notices growing). The
refresher architecture decouples `/` from writes too, so it should stay fast —
but I will re-confirm one burst during **real parsing** (writes) when job 1
passes ~2011 (~100 min out) and record it here as the definitive datum before
this is called done. On current evidence the p99<1s target is met.

## 2026-07-21 16:24 — DEFINITIVE write-load test on b0a5cdb: PASS (run-driver)

Ran the burst during job 1's REAL parsing (pkg 229/2012-02, notices actively
climbing 3028→5844+) — the exact active-write-commit condition that produced the
12–30s stalls + per-request core-pin on bad8dda. Result is a clean pass:

- 8× `curl /` under write commits: **1.8ms–29ms, all HTTP 200** (max 29ms). No
  hangs, no multi-second stalls.
- Immediate pin check: **0 threads >50% CPU**. Box was CPU-saturated by the
  parser (idle 0%, load 5.09) yet `/` still returned in ms — the background
  coverage refresher fully decouples `/` from ingestion. bad8dda pinned a core
  per in-flight `/`; that is gone.
- `/admin/jobs`: 0.7–10.7ms.
- Refresher liveness: `measured_at` advanced 15:10:25Z → 16:11:25Z, so the
  refresher re-runs on later passes (not one-shot). (Underlying coverage values
  stable at this instant because committed data hadn't grown past the re-walk
  point yet — a data-freshness detail, not a latency one.)

**Issue 20's p99<1s-under-ingestion is met.** Per plan I'll re-confirm the same
burst on b978c96 (which carries the issue-38 refresher-timeout hardening) after
its restart, but the core acceptance is demonstrated here.
