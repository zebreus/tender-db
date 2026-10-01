# tender-db watchdogs

Detection-only systemd timers on the production box, plus two units that write.
The watchers log to the journal and nothing else — no paging, no state — so their
whole value is a periodic breadcrumb that a human (or the hourly ownership
check-in) reads when scanning health. A breach also exits the oneshot non-zero, so
it surfaces in `systemctl --failed` — and so does a run that could not look: an
`ERROR` line names a failure to measure and is never folded into `ok` (issue 459).
The two that write (`snapshot`, `tmpsweep`) say so in their rows below; both refuse
to act when they cannot prove the action is safe rather than falling back to a guess.

| unit | fires | checks |
|---|---|---|
| `tender-db-diskwatch` | hourly, `:17` | `/` and `/data` free space vs a used-percent threshold (default 90%). Plain `df`, no call into the server — so it still warns when the disk-full has taken the app down. |
| `tender-db-jobwatch` | hourly, `:29` | `GET /admin/jobs`: any `.recent[]` run with `outcome != "ok"` finished inside the lookback (default 26 h), and any `.current` job running past the wedged threshold (default 8 h, above the ~5 h a full `project rebuild=true` legitimately takes). A run that could not look — no secret, unreachable, any status but 200 (the server's own 403 and 404 are JSON), a 200 without `current`, an array `queued` and an array `recent` — prints `ERROR jobwatch: … — jobs NOT checked` and exits 1, never `ok` (issue 459). |
| `tender-db-driftwatch` | daily, `06:41` | the public SDK-eforms-de release feed (gitlab.opencode.de): warns and exits non-zero the day a release lands beyond the vendored `1.14.x` line — the eForms-DE 2.1 successor whose acceptance deadline is 2026-12-02 (issue 165). A probe that could not look — network failure, any status but 200, a 200 that is not a release list with a tag — prints `ERROR drift: … UNWATCHED this run` and exits 1 as well (issue 459); the line says which of the two it is. It used to WARN and exit 0 so "a flaky mirror never masks a real drift alarm", but the journal held 42 runs, all `ok`, and no failure, while the exit 0 made a dead probe look like "no release yet". If the mirror ever proves flaky, alarm on the age of the last good probe, not on exit 0. |
| `tender-db-snapshot` | weekly, `Sun 05:23` | NOT a watchdog — the one unit here that WRITES: an instant XFS-reflink snapshot of the DB into /data/db/snapshots (issue 269), WAL folded and truncated to the 0-byte sibling the verify suite expects. Waits for the job queue to drain — polling every minute for up to 150 min, because the weekly data-quality run (03:10) overlaps 05:23 whenever it exceeds ~2 h and the 2026-09-13 snapshot was lost to a plain skip (issue 420) — and only then skips loudly; keeps the newest 2 and can never delete the last. Reads the queue through `tender-db-queue-probe.sh` (below) and snapshots only on its named `idle` or `down`: a queue it could not read — wrong or missing secret, a 403/404, a timeout — is polled like a busy one and, if the budget runs out, prints `ERROR snapshot: could not read the queue (…) — NOT snapshotted` and exits 1 (issue 459). Same-volume: a verification/forensics artifact, not disaster recovery. |

| `tender-db-tmpsweep` | daily, `23:41` | WRITES (deletes): removes the turso per-connection temp databases under `/data/tmp` that a restart orphaned (issue 337). `BEGIN IMMEDIATE` makes turso create one `.tmpXXXX/tursodb-temp.db` per connection; the connection's `TempDir` removes it on a graceful drop, but the unit takes a default SIGTERM and never unwinds, so ~1/day escapes and nothing ever reclaims it. Deletes only directories older than the service's `ActiveEnterTimestamp` — provably not held by the running process — and only when their contents are turso's own `tursodb-temp.db*` / `tursodb_temp_file*`. No start time readable ⇒ it abstains. `TENDER_TMPSWEEP_DRY=1` reports without deleting. |

## The queue probe

`tender-db-queue-probe.sh` has no timer. It is the one reading of `GET /admin/jobs`
that two callers act on before doing something a running job would not survive:
`deploy.sh` pipes the repo's copy to the box (`ssh … bash -s <`), so a deploy never
depends on what is installed, and the snapshot runs the installed copy beside it
(`install.sh` puts it in `/usr/local/bin`). It prints exactly one line:

| answer | when | exit |
|---|---|---|
| `idle` | HTTP 200, and the body has a `current` key whose value is null | 0 |
| `busy <id> <kind> <params>` | HTTP 200, and `.current` is an object | 0 |
| `down` | curl exit 7 — nothing accepted the connection (issue 420's "no writer", as a named arm) | 0 |
| `error <what>` | everything else, named: no secret, curl exit N, `HTTP 403 bad or missing operator secret`, `HTTP 404 not found`, a 200 without a `current` key | 1 |

Callers go ahead only on exit 0 with exactly `idle` or `down`. The probes it replaced
printed an empty string for idle **and** for every failure to measure — a missing
secret, a timeout, the server's JSON 403 — so a rotated secret read as an idle box: the
deploy would have restarted a running job, the snapshot reflinked mid-fold (issue 459,
instrument-discipline.md ledger #6). A `403` means `/root/tender-admin-secret` no longer
matches the secret the service read at start; a `404` means the service runs without
`TENDER_ADMIN_SECRET` (the `admin.conf` drop-in is gone).

Thresholds are env-overridable in the unit if needed: `TENDER_DISK_WARN_PCT`,
`TENDER_JOB_FAIL_LOOKBACK_SECS`, `TENDER_JOB_WEDGED_SECS`, `TENDER_ADMIN_URL`,
`TENDER_ADMIN_SECRET_FILE`; for the drift watch, `TENDER_DRIFT_KNOWN_PREFIX`
(bump it when a new SDK-DE line is vendored) and `TENDER_DRIFT_API`.

## Install / restore

```
sudo ops/watchdogs/install.sh
```

Idempotent: copies the scripts to `/usr/local/bin`, the units to
`/etc/systemd/system`, reloads systemd, enables + starts the timers, and dry-fires
each once to prove it runs clean.

## Why they live in git

They didn't, and that was the bug. The scripts were provisioned only on the box.
On **2026-08-09** they disappeared from `/usr/local/bin` (a `/usr/local/bin` clear
around 14:10), and from then the timers failed hourly with `203/EXEC` — a week of
noise in `systemctl --failed` and, worse, **no disk or job-failure detection at
all** during that window, while everyone assumed the watchers were watching. See
issue 224. Keeping the scripts and units in the repo, restorable with one
idempotent `install.sh`, is the fix: a box rebuild or an accidental delete is now a
one-command recovery, and any change to the watch logic is reviewed and versioned.

## Relationship to `/health/deep`

The app's `/health/deep` already reports disk (`disk.ok`, `used_fraction`) and the
last job (`last_job.ok`) and ingest freshness. These timers are the app-independent
complement: `diskwatch` runs even if the server is down, and `jobwatch` sees the
whole recent-run window and the wedged-in-flight case, not just the single last
job. They do not replace `/health/deep`; they cover the gaps it structurally can't.

## Testing them offline

`ops/watchdogs/test-watchdogs.sh` runs the jobwatch, queue-probe, snapshot-gate and
driftwatch cases against a fixture server — no box, no real admin secret, no network,
no root. `install.sh` refuses to install when it fails. Run it after editing any
watchdog:

```
ops/watchdogs/test-watchdogs.sh     # exits 0 on success
```

It exists because issue 373 could only have been caught this way. `jobwatch`
was applying a 26 h failure lookback to the endpoint's DEFAULT depth of 20
runs; on prod that reached back 17.7 h, so a daily failing in the 8.3 h gap left
the journal saying `ok`. Running the script against the real box could not
distinguish a saturated window from a quiet one — both print `ok` — so the
fixture server deliberately honours `?limit=` the way the real endpoint does,
and the cases pin the ARITHMETIC between the lookback and what the script can
actually see.

The harness is checked against its own negative: with the `?limit=` and the
saturation guard removed, the shallow-window case fails with exactly the
historical symptom (`20 recent covering 19h` reported as `ok`). A test that
passed against both the fixed and the broken script would be worth nothing.

The fixture also checks `x-admin-secret` (issue 459). It used to answer 200 to
anything, which held authentication constant — the "both-ways is per-axis" trap in
`docs/agents/instrument-discipline.md` — so a script that never read the HTTP status
passed every case while, on the real box, it read the server's JSON 403 as
`ok jobwatch: idle, 0 queued, 0 recent covering 0h`. Now a wrong secret gets
`deny()`'s real 403 body, a `secret-unset` mode gets its 404, and the cases assert that
jobwatch, the probe, the snapshot and driftwatch each turn those into an `ERROR` and a
non-zero exit, with no `ok` and no `dry run` line beside it. The probe cases run the
probe both as `bash -s <file>` (deploy.sh's bytes, which also pins that nothing in it
reads stdin) and as a file (the snapshot's way) and require the same single line.
Checked against its own negative on 2026-10-01: the new harness run over the
pre-459 jobwatch, snapshot and driftwatch fails 22 cases, and a probe sabotaged to
answer `idle` on a non-200 fails 8.
