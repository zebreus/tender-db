# 224 — the disk/job watchdog timers died 2026-08-09; a week of blind monitoring, now restored + made durable

Status: RESOLVED — restored & made repo-durable 2026-08-16. Both timers were failing hourly with
`203/EXEC` since **2026-08-09** because their scripts (`/usr/local/bin/tender-db-diskwatch.sh`,
`tender-db-jobwatch.sh`) had vanished from the box (a `/usr/local/bin` clear around 14:10 that day); they
had been provisioned ONLY on the box, so nothing in git could restore them. Found during the ownership
check-in while reconciling `/admin/jobs` access — `systemctl --failed` listed exactly these two units.

Fix: reconstructed both scripts (behaviour inferred from the unit descriptions + `model::ingestion` job
shapes), added the four unit files and an idempotent `install.sh`, all under **`ops/watchdogs/`** in the
repo, and installed them on the box. `systemctl --failed` is now clean; both timers dry-fire green.

Kind: observability / operations (monitoring was dark, and the recovery path didn't exist)
Blocked by: —
Relates to: 213 (`/health/deep` real DB check — the app-side half these complement), 133 (the
emptied-layer detector that DOES live in the app)

## What was actually broken

- `tender-db-diskwatch.timer` (`:17` hourly) → `tender-db-diskwatch.service` → missing script.
- `tender-db-jobwatch.timer` (`:29` hourly) → `tender-db-jobwatch.service` → missing script.

Both are `Type=oneshot`, "detection only, journal-visible." For ~7 days every fire exited `203/EXEC`
(executable not found), so: (1) chronic `systemctl --failed` noise that would bury a real failure, and
(2) **no free-space or job-failure detection at all** during the window, while the units' continued
existence implied they were watching. No journal record of a successful run exists in the retained
window, so it is possible they never ran post-provisioning — either way, dark.

`/health/deep` (issue 213) covers disk (`disk.ok`, `used_fraction`) and the single last job
(`last_job.ok`) + ingest freshness, so production was not unmonitored — but it structurally cannot see
the app-down disk case (it is the app) nor the whole recent-run window / wedged-in-flight case. The
watchdogs are that complement, which is why restoring rather than deleting them is correct.

## Root cause (the durable defect, not the symptom)

Provisioning lived only on the box. A hand-installed script under `/usr/local/bin` has no version, no
review, and no recovery path — one `rm` or box rebuild and it is gone silently. The units survived
(they are declarative and were installed once) but pointed at nothing.

## Fix, as shipped

`ops/watchdogs/` now holds:
- `tender-db-diskwatch.sh` — app-independent `df` check of `/` and `/data` vs `TENDER_DISK_WARN_PCT`
  (default 90%, matching `/health/deep`'s `threshold_fraction` 0.9). WARN → non-zero exit → visible in
  `systemctl --failed`.
- `tender-db-jobwatch.sh` — reads `GET /admin/jobs` with the operator secret; WARNs on a `.recent[]`
  run with `outcome != "ok"` finished inside `TENDER_JOB_FAIL_LOOKBACK_SECS` (26 h) and on a `.current`
  job past `TENDER_JOB_WEDGED_SECS` (8 h, above the ~5 h a full `project rebuild=true` legitimately
  takes — id 697 ran 5.1 h — so a healthy rebuild does not false-trip). Clean run logs one OK summary.
- the four `.service`/`.timer` unit files (verbatim copies of what the box runs), and
- `install.sh` — idempotent restore: copy scripts + units, `daemon-reload`, `reset-failed`,
  `enable --now`, dry-fire each. One-command recovery after a rebuild or delete.

## Follow-up (optional)

Fold `ops/watchdogs/install.sh` into whatever provisions the prod box (the deploy path installs the
binary but not these units), so a fresh box comes up with the watchdogs already installed rather than
relying on someone remembering to run it. Low priority — the durable git copy already removes the
silent-loss failure mode.
