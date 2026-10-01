# 461 — `/health/deep` ingest_freshness reads "no ok probe/process in the newest 100 job_log rows" as fresh

Status: ready-for-agent — filed 2026-10-01 from the owner's board survey (workflow wf_4eac8781-4d0, verified by an adversarial pass). The first unit is the fix and its test: read the newest ok `probe`/`process` from `job_log` directly, and keep the fresh-box exemption only for an empty log.
Kind: risk (alerting: a false green on the check the external watchers page on)
Relates to: 226 (closed; accepted this gap), 256 (closed; saw it on prod), 24 (the watchers that read this check),
56 (dormant; the opposite case, a false 503 during a long backfill), 342 (the FTS backfill whose `process` jobs carry
the clock today)

## What is wrong

Code at 9b44528, which is the running rev on prod (`/health/deep` `rev`, 2026-10-01 11:50 UTC). `health.rs`,
`metrics.rs` and `store/src/jobs.rs` are unchanged since f40d5e5.

1. `deep` reads `state.db.recent_job_runs(JOB_SCAN)` with `JOB_SCAN = 100` (`crates/app/src/v1/health.rs:52`, `:86`).
   That is `SELECT … FROM job_log ORDER BY id DESC LIMIT ?` (`crates/store/src/jobs.rs:127-133`).
2. `ingest_last_success` (`health.rs:121-125`) returns the newest ok `probe` or `process` among those 100 rows, or
   `None`.
3. `assess` (`health.rs:162-163`) counts `None` as healthy:

       let age = s.last_success.map(|t| s.now - t);
       let fresh_ok = age.is_none_or(|age| age <= INGEST_STALE_SECS);

So `None` covers two different states:
- **An empty log.** This is a fresh box. The exemption is documented: `docs/operations.md:691-695` says "A box that
  has *never* run a job … is reported healthy-but-unmeasured".
- **A log whose newest 100 rows hold no ok ingest.** This is not missing evidence. It shows that no ingest
  succeeded in the last 100 runs. The check still reports `ok:true`, and `/health/deep` answers 200.

The code contradicts its own comments:
- The test `a_maintenance_success_does_not_reset_ingest_freshness` says at `health.rs:394`: "Only maintenance in the
  window → None: unmeasured, never a false green". At `:423` it says "a window holding only the refold pair is
  unmeasured, never fresh". `assess` turns that `None` into `ok:true`. No test passes a `None` with runs present
  through `assess`. `a_box_with_no_runs_yet_does_not_alarm` (`:514`) covers only `last_job: None`.
- The doc on `Signals.last_success` (`:134`) says "`None` if none ever has". That is wrong: it is also `None` when
  no ok ingest is in the window.
- The doc on `JOB_SCAN` (`:48-50`) argues that a run longer than the window is caught by the last-job check. That
  holds only for a run of failures. A run of successful non-ingest jobs trips nothing.
- `/metrics` reads the same window (`crates/app/src/v1/metrics.rs:331-339`), so the
  `tender_db_ingest_last_success_timestamp_seconds` gauge disappears in the same case. Its HELP text still lists
  `project`, which was removed from `INGEST_KINDS`.

The board already saw this and did not fix it:
- **226 (closed)**, lines 51-53, accepted the gap: "unless 100+ non-ingest jobs have run since — itself an abnormal,
  separately-visible state". It is not separately visible. `last_success_at: null` sits inside an HTTP 200 with
  `ok:true`, and the GitHub watcher looks only at the status code and the top-level `ok`
  (`.github/workflows/uptime-check.yml:52`, `:56`).
- **256 (closed 2026-08-21)**, lines 61-67, recorded this on prod on 2026-08-20, the day the daily tick never
  enqueued: `"ingest_freshness": {"age_secs": null, "last_success_at": null, "ok": true, ...}`. In 256's words, "it
  is the check that says fine". 256 fixed the enqueue, not this check.

**The window is not "a month" any more.** That was 226's premise. Rows per UTC day from
`GET /admin/jobs?limit=300`, read on the box 2026-10-01 11:4x UTC:

| days | rows/day |
|---|---|
| 2026-09-21 … 09-26 | 7–9 |
| 2026-09-27 … 09-29 | 31–39 |
| 2026-09-30 | 86 |
| 2026-10-01 (to 11:35) | 40 |

The 100th-newest row is log row 2609 (job 1700, `reveal-recheck`), finished 2026-09-30 07:36:24Z. So today's window
spans about 28 h, two hours more than the 26 h threshold. Of the last 300 rows, 27 are `match-org-identifiers` runs.

Two ways this pages no one:
1. **False recovery.** The daily stalls, as it did in 256. At 26 h the check reads `ok:false`, the endpoint answers
   503, and the GitHub watcher opens an `uptime` issue. Then about 100 runs of other kinds (an org campaign, censuses,
   repairs, refolds) push the last ok ingest out of the window. `last_success` becomes `None`, the check reads
   `ok:true`, and the watcher closes its issue with "Recovered at …" (`uptime-check.yml:76`), although nothing has
   been ingested.
2. **No alarm at all.** This happens when 100 or more successful non-ingest runs finish within 26 h of the last ok
   ingest. 2026-09-30 logged 86 rows of all kinds.

Not in scope here: the clock currently reads `last_success_at` 1790854384 (11:33:04Z). That is job 1797,
`process fts monthly (all)`, the 342 backfill chunk, not the 07:36–07:38Z daily (jobs 1767–1772). Whether a
backfill's `process` should count as arrival is a separate question.

## Proposed fix

The root cause is that freshness reads a window sized for other questions (the last job's outcome, the per-kind
gauges), so "not in the window" is read as "never happened". The fix is to ask the log the freshness question
directly.

1. **Store.** Add `Db::last_ok_run_finished(kinds: &[&str]) -> turso::Result<Option<i64>>`, which runs
   `SELECT max(finished_at) FROM job_log WHERE outcome = 'ok' AND kind IN (…)` through the reader pool, as
   `recent_job_runs` does. `job_log` has only its primary-key index. It has at most 2,708 rows (newest log id,
   2026-10-01), and nothing in `crates/` deletes from it, so the scan is trivial.
2. **Callers.** `deep` and `/metrics` both read the clock from that query, and `ingest_last_success(&runs)` is
   removed. `recent_job_runs(JOB_SCAN)` stays for `last_job`, the database probe and the per-kind gauges.
3. **Verdict.** `None` counts as fresh only when the log is empty. `deep` already has that fact as `last_job`:

       let fresh_ok = match (s.last_success, &s.last_job) {
           (Some(t), _) => s.now - t <= INGEST_STALE_SECS,
           (None, None) => true,     // empty log: a fresh box, unmeasured
           (None, Some(_)) => false, // runs, but no ok ingest ever: stale
       };

   The accepted cost: a new box where an operator runs a maintenance job before the first daily reads `ok:false`
   until that daily lands. That reading is true.
4. **Docs.** Correct the comments this contradicts: `Signals.last_success` (`:134`), `JOB_SCAN` (`:48-51`),
   `assess` (`:155-158`), the `/metrics` HELP text (`metrics.rs:336`), and `docs/operations.md:691-695`, which says
   "no job has succeeded" where it means no `probe` or `process`.

**Tests that pin it:**
- Integration test in `crates/app/tests/api.rs`, `a_window_full_of_other_runs_does_not_hide_a_stale_ingest`:
  - Record an ok `probe` that finished 27 h ago, then 101 newer ok `fetch` runs.
  - `/health/deep` must answer 503, with `ingest_freshness.ok == false` and `last_success_at == now - 27 h`.
  - Repeat with the probe 1 h ago. The check must read `ok:true` with that real age. Today it reads `ok:true` with
    `last_success_at: null`.
  - `/metrics` must carry the gauge in both cases.
- Unit test in `health.rs`, `runs_without_an_ingest_are_stale_not_unmeasured`: `last_success: None` with
  `last_job: Some(..)` must give `ingest_freshness.ok == false`.
- `a_box_with_no_runs_yet_does_not_alarm` and the fresh-box half of `the_deep_health_probe_reports_operational_health`
  (`api.rs:389`) stay unchanged. They pin the empty-log exemption.

## Verify

    ssh -o BatchMode=yes root@zebreus.click "git -C /opt/tender-db/src rev-parse --short HEAD; grep -c 'fn a_window_full_of_other_runs_does_not_hide_a_stale_ingest' /opt/tender-db/src/crates/app/tests/api.rs"

- **open** (2026-10-01 11:5x UTC; re-read 13:26 UTC, same): `9b44528` and `0`. The deployed source has no such
  test, and `assess` still turns `None` into `ok:true`.
- **done:** `1`, with the deployed rev at or after the 461 commit. The gate runs the test, so it proves the
  behaviour. Prod's own `/health/deep` reads the same in both states while dailies land, so the deployed source is
  the only thing this check can see.
