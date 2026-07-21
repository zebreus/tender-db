# 20 — read-only queries queue behind the writer under heavy ingestion

Status: needs-verification

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
