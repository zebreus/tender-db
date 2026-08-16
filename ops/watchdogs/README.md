# tender-db watchdogs

Two hourly, detection-only systemd timers on the production box. They write to the
journal and nothing else — no paging, no state — so their whole value is a periodic
breadcrumb that a human (or the hourly ownership check-in) reads when scanning
health. A breach also exits the oneshot non-zero, so it surfaces in
`systemctl --failed`.

| unit | fires | checks |
|---|---|---|
| `tender-db-diskwatch` | hourly, `:17` | `/` and `/data` free space vs a used-percent threshold (default 90%). Plain `df`, no call into the server — so it still warns when the disk-full has taken the app down. |
| `tender-db-jobwatch` | hourly, `:29` | `GET /admin/jobs`: any `.recent[]` run with `outcome != "ok"` finished inside the lookback (default 26 h), and any `.current` job running past the wedged threshold (default 8 h, above the ~5 h a full `project rebuild=true` legitimately takes). |

Thresholds are env-overridable in the unit if needed: `TENDER_DISK_WARN_PCT`,
`TENDER_JOB_FAIL_LOOKBACK_SECS`, `TENDER_JOB_WEDGED_SECS`, `TENDER_ADMIN_URL`,
`TENDER_ADMIN_SECRET_FILE`.

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
