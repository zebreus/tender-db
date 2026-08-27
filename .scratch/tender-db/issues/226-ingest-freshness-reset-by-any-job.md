# 226 — `/health/deep` ingest_freshness was reset by ANY successful job, so a maintenance run masks a stalled ingest

Status: CLOSED (prod-checked 2026-08-27 21:5x: after two reindex-bearing
deploys + restarts that evening, /health/deep ingest_freshness.last_success_at
still read 1787816201 — that morning's daily pipeline run — with ok:true; the
fix holds under exactly the trigger that filed this issue). Was:
RESOLVED-DEPLOYED (be4c96c, live on prod rev ae31cbd; board-hygiene sweep 301,
2026-08-27). Found during the ownership check-in: a manual `reindex`
(217-B, job 703) reset `ingest_freshness.last_success_at` to its own finish time even though it is not an
ingest. `deep()` now filters `last_success` to the daily-pipeline kinds (probe/process/project); a
maintenance job can no longer report the box "fresh."

Kind: observability (a health check measuring the wrong thing)
Blocked by: —
Relates to: 213 (the deep probe's real-DB check), 24 (alerting), 222 (the morning catch-up this freshness
signal watches over), the jobwatch watchdog (issue 224)

## Defect

`crates/app/src/v1/health.rs` computed the freshness clock as:

```rust
let last_success = runs.iter().find(|r| r.outcome == "ok").map(|r| r.finished_at);
```

`runs` is the newest-first job log, so `last_success` was **the newest successful run of ANY kind**. The
check is named `ingest_freshness` and its threshold (26 h) is sized for the daily 09:35 pipeline, but it
actually measured "time since any job succeeded." So a `reindex`, `reprocess`, or `refold` — maintenance
jobs the owner runs by hand during ops — resets the clock and reports the box fresh while the daily
probe/process/project pipeline may have stalled. Demonstrated live: right after the 217-B reindex,
`/health/deep` showed `last_success_at` = the reindex's finish, not the 09:35 project's.

The window makes it worse in exactly the situation you'd want the alarm: a burst of maintenance activity
(a recovery, a backfill, a reprocess sweep) is precisely when an ingest stall is easy to cause and most
in need of catching — and it is also when the most non-ingest successes pile up to mask it.

## Fix

Filter to the daily-pipeline kinds:

```rust
const INGEST_KINDS: [&str; 3] = ["probe", "process", "project"];
fn ingest_last_success(runs: &[JobRun]) -> Option<i64> {
    runs.iter().find(|r| r.outcome == "ok" && INGEST_KINDS.contains(&r.kind.as_str()))
        .map(|r| r.finished_at)
}
```

`probe` runs every day regardless of new packages, so it is the reliable heartbeat; `process`/`project`
only run when there is work, but their success is equally an ingest. `last_job` (the "newest run errored?"
check) stays any-kind on purpose — it reports the most recent run whatever it is. `JOB_SCAN` is 100 (~a
month of dailies), so the filtered search reliably finds the last ingest unless 100+ non-ingest jobs have
run since — itself an abnormal, separately-visible state.

## Verification

- Unit test `a_maintenance_success_does_not_reset_ingest_freshness`: a newer `reindex`/`reprocess` ok does
  not shadow an older `process` ok; only-maintenance → `None` (unmeasured, not a false green); a failed
  ingest does not count but the prior success does.
- Post-deploy: run a `reindex`, then `/health/deep` — `ingest_freshness.last_success_at` stays the last
  daily run, not the reindex.
