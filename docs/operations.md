# Operations runbook

Production is a single Hetzner VPS, `root@zebreus.click`, serving
<https://tenders.zebreus.click>. It runs Ubuntu, not NixOS — the app is built by
the flake and run under a hand-written systemd unit that mirrors
`nix/module.nix`'s hardening flags (ADR-0006). Design context: `CONTEXT.md`.

## Layout on the box

| Path | What |
| --- | --- |
| `/opt/tender-db/repo.git` | bare repo; `git push vps main` from the dev machine lands here |
| `/opt/tender-db/src` | working clone the build runs from |
| `/opt/tender-db/app` | symlink → the live nix store path (atomic switch on deploy) |
| `/opt/tender-db/app-result` | `nix build` result link (the most recently built bundle) |
| `/opt/tender-db/deployed-rev` | git rev of the running deploy |
| `/data/db/tender-db.db` | the Turso database (500 GB volume) |
| `/data/archive/<source>/…` | raw fetched packages, immutable |

`/opt/tender-db/` also holds research artifacts from the exploration phase
(sample packages, SDK checkouts, scan scripts). They are not part of the
deployment; leave them alone.

## Deploy

From a clean checkout on the dev machine:

```sh
./deploy.sh          # deploys main
./deploy.sh <ref>    # deploys any ref
```

> **A deploy builds from a COMMITTED SHA in a CLEAN tree — never the shared working
> tree.** *"From a clean checkout"* above is the rule, not a stylistic preference, and
> this is what it prevents.
>
> Several agents share this worktree. On 2026-08-05 a deploy was authorised at
> `80e2365`; `HEAD` was `80e2365`, and the tree was **not** — two files carried
> uncommitted work, and they were the two files the deploy shipped
> (`crates/app/src/supervisor.rs`, `crates/store/src/lib.rs`). A teammate was mid-edit
> on the very constant the deploy's review had cleared. Building "what's here" would
> have put un-committed, un-reviewed code into production, and a deploy is the one
> action where that is unrecoverable.
>
> So, before staging:
>
> ```sh
> git status --short crates/     # MUST be empty
> git log --oneline -1           # the SHA you are deploying, read from output
> ```
>
> If the tree is dirty, deploy from a fresh checkout of the SHA instead — never
> `git stash` someone else's work to clear the path.
>
> **Two corollaries, both learned the same day.** A pre-deploy test run against a dirty
> tree certifies a tree nobody will deploy — the green describes the wrong artifact, so
> run the gate on the committed tip. And **a review clearance is bound to the artifact it
> was given against**: if the code changes after a review, the clearance does not
> automatically follow it, and the reviewer has to say so. See
> [`agents/instrument-discipline.md`](agents/instrument-discipline.md).

The script pushes the ref to the VPS bare repo, builds `#tender-db` **on the
VPS** (it has the 1 Gb/s uplink and the warm nix store — never build the bundle
over the dev machine's ~100 kB/s link), atomically switches `/opt/tender-db/app`,
restarts the service, and health-checks the public URL. A failed build never
touches the symlink, so the old bundle keeps serving. The ssh connection carries
keepalives (`ServerAliveInterval=15`, `ServerAliveCountMax=4`) so a dropped TCP
link fails the deploy cleanly instead of hanging forever on a dead socket
(2026-07-21 incident).

Build times, warm store (the normal case): a **server-code-only** change
rebuilds in **~4.6 min**; a change that touches the **dependency graph**
(`Cargo.lock`, a new crate, a toolchain bump) is **~10.5 min**. A first build on
a *cold* store compiles the whole Rust + wasm toolchain graph and takes far
longer (tens of minutes on the box's 4 cores). If a deploy might outlive your
connection, run it inside tmux on the box.

**Deploys are one at a time and never move production backwards** (f8bed0b —
two concurrent deploys raced once and the slower one would have regressed the
live rev). The VPS-side critical section (build → symlink switch → restart)
takes a `flock` on `/opt/tender-db/deploy.lock`; a second deploy while one holds
it aborts immediately with

```
another deploy holds /opt/tender-db/deploy.lock — aborting
```

Inside the lock it also refuses a regression: if the target rev is an ancestor
of the currently deployed rev (recorded in `/opt/tender-db/deployed-rev`) it
exits with `refusing regression: <rev> is an ancestor of deployed <rev>`. So
redeploying an older commit is a deliberate act — check out or revert to a
descendant rather than pointing `deploy.sh` at the old one. (For an emergency
revert to a *known-built* older bundle, use the symlink rollback below, which
bypasses the build and the guard.)

The health check at the end probes `GET /health` (see [Ingestion](#ingestion));
it must return `"ok":true` throughout, since ingestion runs in-process and the
readers keep serving over WAL during a load. The script polls for up to **120 s**
(one probe/second) before it declares the deploy failed — that grace exists
because the **first open after a schema change migrates**: additive `ALTER`s plus
`CREATE INDEX IF NOT EXISTS` build once over the whole table (tens of seconds on
the multi-million-row `notices`), which can hold `/health` past a minute. That is
startup work, not failure — the migrating-open pause clears itself once the index
is built and every subsequent open is instant.

The deploy also writes the rev into a systemd drop-in
(`/etc/systemd/system/tender-db.service.d/rev.conf`,
`Environment=COMMIT_SHA=<rev>`) and reloads before the restart. The app reads
`COMMIT_SHA` at runtime (`crates/app/src/v1/mod.rs`, `rev()`), so `/health`,
`/v1`, `/_source`, and the dashboard's System panel report the actual deployed
revision — while the `nix build` never sees the rev and stays reproducible. A
plain local build (no `COMMIT_SHA` in the environment) reports `dev`.

Rollback: point the symlink at a previous store path and restart.

```sh
ssh root@zebreus.click 'ln -sfnT /nix/store/<old-path> /opt/tender-db/app.new \
  && mv -T /opt/tender-db/app.new /opt/tender-db/app && systemctl restart tender-db'
```

Old store paths survive until `nix store gc` runs; `nix profile`-style
generations are deliberately not used — the symlink is the whole mechanism.

## Service management

```sh
systemctl status tender-db
systemctl restart tender-db
systemctl stop tender-db
systemctl is-enabled tender-db      # enabled → survives reboot
```

Unit: `/etc/systemd/system/tender-db.service`, plus drop-ins under
`/etc/systemd/system/tender-db.service.d/` (`systemctl cat tender-db` shows the
merged result). Runs as the `tenderdb` system user (not `DynamicUser`, unlike
the NixOS module — the data in `/data` must outlive restarts).
`ProtectSystem=strict` with `ReadWritePaths=/data/db /data/archive`: the process
can write nowhere else, so a new state directory needs a unit edit, not just a
`mkdir`. The base unit sets `IP=127.0.0.1`, `PORT=8080`, `TENDER_DB`,
`TENDER_ARCHIVE`; the operator secret comes from the `admin.conf` drop-in — see
[Ingestion](#ingestion).

Anything writing to `/data` outside the service (a manual fetch run, say) must
leave the files owned by `tenderdb`, or the service loses access:

```sh
chown -R tenderdb:tenderdb /data/db /data/archive
```

## Ingestion

Ingestion runs **inside the server process** (ADR-0005, issue 16): the
Supervisor is a background task that owns the writer for its jobs while the
readers keep serving over WAL, so a production load has **zero downtime** and no
external process ever opens the DB (turso is single-process). You never stop the
service to load data.

Two ways jobs start:

- **Scheduler** — at 09:35 Europe/Berlin it enqueues the daily pipeline: on
  Mon–Fri a TED probe forward + re-fetch of the current day (the 09:30 CET
  finality window) and a `process`; every day a DÖE completed-day fetch (T+1,
  yesterday's date) + `process`; then one `project` that folds whatever landed.
  (The trailing `snapshot` job was removed with the backup feature,
  2026-08-06.) No operator action needed. Confirm a run fired by looking for a
  `probe` job (and the trailing
  `fetch`/`process`/`project`) with that morning's `started_at` in
  `GET /admin/jobs` → `recent[]`, or on the dashboard's Ingestion panel. The
  scheduler is a plain in-process timer (no cron/systemd timer), so it only runs
  while the service is up — a box that was down at 09:35 simply misses that tick;
  re-drive it by hand via `/admin` if needed.
- **`/admin` API** — for manual loads, backfills and reprocessing. Gated by a
  preshared operator secret in `TENDER_ADMIN_SECRET`, sent as the
  `X-Admin-Secret` header and compared in constant time. **Unset ⇒ the whole
  `/admin` surface answers 404** (the feature is simply absent); a wrong secret
  is 403.

> ⚠️ **The `fetch` / `process` / `project` CLIs are dev tools for scratch
> databases only.** Never run them against the production DB: turso is
> single-process, so a CLI cannot open the file while the service is running, and
> stopping the service to run one is exactly the downtime this design removes.
> Everything below goes through `/admin` instead.

### The operator secret

Live setup on the box (never committed): the secret is an `EnvironmentFile`
holding one `KEY=VALUE` line, kept out of the main unit so it never appears in
`git` or `systemctl cat` of the checked-in unit.

```
/root/tender-admin-secret                          # mode 600, root-owned:
    TENDER_ADMIN_SECRET=<64 hex chars>

/etc/systemd/system/tender-db.service.d/admin.conf # the drop-in that wires it:
    [Service]
    EnvironmentFile=/root/tender-admin-secret
```

systemd reads the `EnvironmentFile` as root at start, before dropping to the
`tenderdb` user, so the `0600 root` file stays unreadable to the service user
and everyone else. First-time setup:

```sh
# On the VPS:
printf 'TENDER_ADMIN_SECRET=%s\n' "$(openssl rand -hex 32)" > /root/tender-admin-secret
chmod 600 /root/tender-admin-secret
mkdir -p /etc/systemd/system/tender-db.service.d
printf '[Service]\nEnvironmentFile=/root/tender-admin-secret\n' \
  > /etc/systemd/system/tender-db.service.d/admin.conf
systemctl daemon-reload && systemctl restart tender-db
journalctl -u tender-db -n5   # logs "admin: /admin API enabled"
```

Rotating it is an edit of the file + `systemctl restart tender-db`; the secret
lives only on the box. Unset it (remove the drop-in) and the entire `/admin`
surface goes back to answering 404.

### Driving it (examples)

`GET /admin/jobs` returns the running job's live progress, the queue, and the
recent-run log — the same shape the dashboard's Ingestion panel renders.

```sh
SECRET=$(cat /root/tender-admin-secret)
BASE=https://tenders.zebreus.click

# What is the importer doing right now?
curl -s -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs | jq

# Fetch one TED daily, then process + project it (three sequential jobs).
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"fetch","source":"ted","package_kind":"daily","period":"2026-00136"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"process","source":"ted","package_kind":"daily","period":"2026-00136"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"project"}' $BASE/admin/jobs

# Cancel a job. Two verbs reach the same handler (issue 250) — the POST form exists
# because a DELETE is unreachable from some operating sessions.
curl -s -XPOST -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs/41/cancel
curl -s -XDELETE -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs/41
# …or, on the box: ops/admin.sh cancel 41
#
# Four answers (issue 252):
#   200 {"state":"dropped"}   it was queued and is gone
#   200 {"state":"stopping"}  it is running and its kind checks the stop flag
#   409                       it is running as a kind with NO stop checkpoint — the
#                             honest refusal. The stoppable set is STOPPABLE_KINDS in
#                             supervisor.rs (a contract test pins it): reparse,
#                             data-quality, project, merge-provisional-orgs,
#                             org-merge-health, r2-census, r3-census,
#                             match-org-identifiers, build-org-match-keys,
#                             scan-org-match-keys, org-edge-census,
#                             case-review-backlog, fold-org-countries,
#                             fusion-census, rehoming-packet,
#                             satellite-orphans
#   404                       no such job
# A cancelled data-quality run stores NOTHING: a half-measured report would read like a
# whole-corpus one, so the previous report stands.

# Backfill a DÖE monthly range (fans into one fetch per month + process + project).
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"backfill","source":"doe","range":["2024-01","2024-12"]}' $BASE/admin/jobs

# Re-fold every notice carrying a section of a KIND (issue 237) — the cheap cohort:
# notice_sections_kind is indexed, so this is an index read, unlike refold-fields.
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"refold-sections","profiles":["GroupComposition"]}' $BASE/admin/jobs

# Re-fold an explicit, small notice-id list (issue 58's step-3 exerciser). Capped
# at 1,000 ids: a longer list is a cohort and wants `refold`/`refold-fields`.
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"refold-notices","notices":[123,124,125]}' $BASE/admin/jobs

# Cancel a still-queued job (the running one cannot be cancelled).
curl -s -XDELETE -H "X-Admin-Secret: $SECRET" $BASE/admin/jobs/42

# Read the newest stored data-quality report (issue 230). The measurement is a
# weekly job — Sunday 03:10 Berlin, ~36 min over 32 id windows — and this is
# where its body lands. `age_seconds` is served so a stale report cannot be
# mistaken for a current one.
curl -s -H "X-Admin-Secret: $SECRET" $BASE/admin/reports/data-quality | jq -r .body
curl -s -H "X-Admin-Secret: $SECRET" $BASE/admin/reports/data-quality | jq '{computed_at, age_seconds}'
```

On the box: `tender-admin raw GET /admin/reports/data-quality </dev/null | jq -r
.body`. A kind nothing has computed yet answers 404, not an empty report — "not
measured" and "measured as zero" are different claims and the report is careful
about the difference (its own text banners any section it could not measure).

Job payloads (`crates/app/src/supervisor.rs`, `JobRequest`): `{kind:
fetch|process|project|backfill|daily|reprocess|reindex|refold|refold-fields|
refold-notices|refold-sections|reparse|data-quality|backfill-titles|mark-skipped-siblings|clear-rebuild-flag,
source?, package_kind?, period?, range?, rebuild?, refetch?, profiles?, notices?,
expect?, dry_run?}` (the `snapshot` kind was removed 2026-08-06).
Defaults: `source` `ted`, `package_kind` `daily`.
`process`/`project` accept no period to run the whole source (`process` with no
`period` re-parses every archived package of that source). `refetch:true`
re-downloads a known package (finality re-check). `project` with `rebuild:true`
drops and re-derives the whole canonical layer. `backfill` needs a `source`; for
`ted` it also needs a monthly `range` (`["2024-01","2024-12"]`), for `doe` the
range is optional (defaults to the whole 2022-12→now archive). A backfill fans
into one `fetch` per month, then one whole-source `process`, then one `project`,
so progress and cancellation stay per-package. Jobs run **one at a time** in
enqueue order — the writer is single anyway — so a fetch → process → project
sequence lands in order.

### The organization-layer jobs (issues 300, 311-317)

These are their own family: censuses that measure, merge arms that write, and
review machinery that records verdicts. **Every writing one defaults to
`dry_run: true`** — a forgotten flag must mean the harmless thing — and the
merge arms run the T4 ladder (dry census → recorded plan → capped wet →
uncapped wet), so a wet run refuses unless its dry plan is on file.

| kind | writes? | notes |
|---|---|---|
| `org-merge-health`, `r2-census`, `r3-census` | no | the standing measurements |
| `match-org-identifiers` (`rule: r2` / `r3`) | YES | the merge arms; `max_groups` caps a run |
| `build-org-match-keys` | satellite only | wholesale rebuild, ~85 s; **weekly since issue 315** |
| `scan-org-match-keys` | edges only | the E3 candidate scan and tripwire 6's clock; weekly |
| `org-edge-census` | no | sizes the edge store into review cohorts (issue 314) |
| `apply-case-reviews` / `unapply-case-reviews` | YES | the issue-311 verdict applier and its undo |
| `case-review-backlog` | no | the parked verdicts nobody consumes (issue 317) |
| `fusion-census` | no | which reviewed rows hold mentions naming somebody else (317 Unit A) |
| `rehoming-packet` | no | the reviewer's input: those mentions, addressed, with destinations |
| `apply-rehoming` | YES | moves a reviewed mention to the row it names; **refold after** |
| `satellite-orphans` | no | name variants a re-homing left behind (issue 321) |
| `fold-org-countries` | YES | backfills non-canonical country codes (issue 319) |

Two refusals an operator will meet, both deliberate:

- **`match-org-identifiers` r3, wet, refuses** when `org_match_keys` is empty
  or a build is mid-walk. R3's corroboration consults the generic-name wall
  (issue 316), and an unbuilt satellite makes every key look unique — the
  wall would silently not be there. The remedy is a **wet** build:
  `{"kind":"build-org-match-keys","dry_run":false}`. The default is dry, and
  a dry build stores nothing, so the flag is the whole remedy.
- **`scan-org-match-keys` refuses** on a missing covering index, a keys-epoch
  mismatch after a deploy, or a missing build report. Each refusal writes
  `org-edge-scan-alarm`; read it with
  `GET /admin/reports/org-edge-scan-alarm`.

### Restarts and recovery (no manual re-enqueue)

The job queue is **durable** (ADR-0007): every outstanding job is a row in
`job_queue`, written before it enters the in-memory queue and deleted only when
the job concludes or is cancelled. So a deploy, crash, or restart **does not lose
the queue** — the Supervisor rebuilds it from the durable rows at startup, before
the worker or scheduler run, and the job that was mid-run when the process died
comes back at the front and re-runs. **You never re-enqueue by hand after a
restart.** Re-runs are safe because ingestion is idempotent (notice identity is
`(source, publication_id, content_hash)`; the projection is a pure function of
the parsed layer), so a repeated walk inserts nothing new.

A long `process` job also carries a **resume cursor** (issue 32): the last
package it fully committed, advanced only after every member of that package
landed. On restart it resumes at the *next* package instead of re-walking years
of archive from the start — the interrupted (partial) package re-runs and dedups.
A *freshly enqueued* `process` never inherits a cursor, so a deliberate whole-
source reprocess still walks everything. The only operator-visible trace is a log
line: `job N resumes after <period> (K package(s) already done)`.

The daily scheduler is a plain in-process timer, so a box that was **down** at
09:35 misses that tick entirely (not a queued job to recover — it never fired);
re-drive it by hand via `/admin` if needed.

### Quarantine triage

A notice whose file matches no mapping profile (or fails a completeness check)
is **quarantined** rather than dropped (ADR-0004): the raw payload and a reason
are kept, and the rest of the package still ingests. Quarantine is the parser's
backlog, not data loss — the archive is intact, so a fixed parser recovers every
quarantined notice by reprocessing.

Triage loop:

1. **See it.** The dashboard (`/`, public) Data-quality panel shows the
   quarantine total, a breakdown by reason, and a recent sample (50). The same
   numbers ride each `process` job's `counts` line in `GET /admin/jobs` →
   `recent[]` (e.g. `… 3702 parsed, 15 quarantined …`).
2. **Diagnose.** Read the reason and the sampled payloads to find the unmapped
   element / customization ID / era the profile doesn't yet handle.
3. **Fix the parser** in `crates/ingest` (a new profile mapping, an ignore rule,
   or an inventory extension), with a fixture test, and **deploy** it
   (`./deploy.sh`).
4. **Reprocess.** Enqueue a `process` for the affected source (no `period` =
   the whole source) then a `project`. Processing re-parses from the archive and
   never re-downloads, so this is cheap and idempotent; quarantined notices that
   the new parser understands become canonical, and the quarantine count drops.

```sh
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"process","source":"ted"}' $BASE/admin/jobs
curl -s -XPOST -H "X-Admin-Secret: $SECRET" -H 'content-type: application/json' \
  -d '{"kind":"project"}' $BASE/admin/jobs
```

## Logs

```sh
journalctl -u tender-db -f              # follow
journalctl -u tender-db -n 200          # recent
journalctl -u tender-db --since '1 hour ago'
journalctl -u tender-db -p err          # errors only
```

nginx: `/var/log/nginx/access.log`, `/var/log/nginx/error.log`.

## Monitoring and alerting

The goal (issue 24) is that the operator learns when production breaks without
watching dashboards. Two health endpoints, and one external watcher that must
live **off the box** (so it still fires when the app — or the whole VPS — is
down). The watcher today is a Claude scheduled routine polling every ~4 h — see
[The off-box watcher](#the-off-box-watcher-an-external-claude-scheduled-routine).

### The two health endpoints

| Endpoint | Cost | Answers | Used by |
| --- | --- | --- | --- |
| `GET /health` | no DB access, always fast | process is up + serving HTTP — liveness only, does **not** query the DB (`{"ok":true,…}`) | `deploy.sh`'s post-deploy check |
| `GET /health/deep` | one job-log scan (also the DB-answer read) + one `statvfs` | liveness **plus** a real DB-answer check, ingest freshness, last-job outcome and disk usage | the external watcher routine |

(A third endpoint, [`GET /metrics`](#get-metrics--the-prometheus-scrape-issue-53),
serves the same signals as *time series* for trend-watching rather than as a
pass/fail verdict.)

`/health` is deliberately narrow: its `ok` reflects only liveness, so a deploy
is never failed by a stale-ingest or full-disk condition unrelated to the new
build. **Do not widen it** — `deploy.sh` greps its `ok:true`.

`/health/deep` (`crates/app/src/v1/health.rs`) folds four signals into one
verdict, so a single external check covers uptime, freshness, job failures and
disk:

- **database** — a real reader-pool read: the job-log scan below serves over WAL
  (issue 20), so its success is the "the database answered" signal and an error
  flips this check unhealthy. `/health` itself is liveness-only and does not touch
  the DB (issue 61/213).
- **ingest_freshness** — unhealthy when no job has succeeded in **26 h**
  (`INGEST_STALE_SECS`). The scheduler lands a successful run at least daily
  (TED Mon–Fri, DÖE + projection every day), and 26 h carries a Friday success
  across the weekend. A box that has *never* run a job (fresh deploy, scheduler
  not yet fired) is reported healthy-but-unmeasured, not alarmed.
- **last_job** — unhealthy when the newest finished run in `job_log` has
  `outcome = "error"` (a Supervisor job ERRORED). Clears itself on the next
  success.
- **disk** — unhealthy once **90 %** (`DISK_FULL_FRACTION`) of the volume
  holding `TENDER_DB` (`/data`, the 500 GB Hetzner volume — same filesystem as
  the archive) is in use. The check also reports `wal_bytes`, the size of the
  `-wal` sidecar (issue 42): **informational only** — a large WAL is expected
  mid-backfill and never flips the verdict — but the alerting routine watches it
  for a runaway (see [Disk watch](#disk-watch)).

It answers **200** when every check passes and **503** when any fails, and the
JSON body names which check tripped:

```sh
curl -s https://tenders.zebreus.click/health/deep | jq
```

```json
{
  "ok": true,
  "rev": "…",
  "checks": {
    "database":         { "ok": true, "cursor": "12345" },
    "ingest_freshness": { "ok": true, "last_success_at": 1753000000, "age_secs": 3600, "threshold_secs": 93600 },
    "last_job":         { "ok": true, "kind": "project", "params": "rebuild=false", "outcome": "ok", "finished_at": 1753000000 },
    "disk":             { "ok": true, "used_fraction": 0.041, "free_bytes": 479000000000, "total_bytes": 500000000000, "wal_bytes": 3500000000, "threshold_fraction": 0.9 }
  }
}
```

### `GET /metrics` — the Prometheus scrape (issue 53)

A third endpoint, and the one for *trends* rather than verdicts:
`/metrics` (`crates/app/src/v1/metrics.rs`) serves the operational levels in
Prometheus text format. It exists because the numbers we hand-watch — WAL size,
RSS, per-job duration, quarantine counts — are all "watch this move over time"
questions, and eyeballing a dashboard answers them hours late. The 2026-07-22/23
WAL incident is the case in point.

```sh
curl -s https://tenders.zebreus.click/metrics | head -20
```

**It is a scrape, not a measurement.** Every gauge is either O(1) (change
cursor, RSS from `/proc/self/status`, live SSE streams, one `statvfs`, the same
bounded job-log window `/health/deep` reads) or a read of the **dashboard's
60-second cache** (canonical row counts, quarantine totals and per-reason
breakdown, import lag). No table-proportional scan runs on this path — that rule
is what keeps a 15-second scrape interval from becoming the load it exists to
observe, and it is the same discipline `coverage.rs` applies to its own heavy
sections.

**A gauge nobody has measured yet is ABSENT, never zero.** On a fresh box, or
while the dashboard's heavy sections are still gated behind a running write job,
the corresponding series simply do not appear. A scraper sees them start when
the first real measurement lands. This is deliberate: a fabricated `0` for
`quarantine_outstanding` reads exactly like a drained backlog.

Series exposed (all gauges — a restart cannot reset a counter mid-series
because nothing here accumulates in-process):

| Prefix | Source |
| --- | --- |
| `tender_db_change_cursor`, `tender_db_sse_streams`, `tender_db_rss_bytes` | in-process, O(1) |
| `tender_db_disk_{used_fraction,free_bytes,total_bytes}`, `tender_db_wal_bytes` | one `statvfs` + the `-wal` stat (same as `/health/deep`) |
| `tender_db_job_last_{duration_seconds,finished_timestamp_seconds,ok}{kind=…}` | newest run per kind in the bounded job-log window |
| `tender_db_ingest_last_success_timestamp_seconds`, `tender_db_ingest_{fetch,notice}_age_seconds` | the freshness clock (`probe`/`process`/`project` only) + import lag |
| `tender_db_canonical_rows{table=…}`, `tender_db_quarantine_*` | dashboard cache (absent until measured) |
| `tender_db_legacy_adjacency_watermark` | one-row point read on the reader pool |

One gauge is deliberately exempt from the absence rule above:
`tender_db_legacy_adjacency_watermark` reports **0** rather than vanishing,
because 0 is a real state there — "legacy OJS adjacency coverage was never
established", in which every legacy delta takes the full-projection fallback
instead of the scoped closure walk (issue 58 v2). It is also the *only* external
view of that claim: the adjacency tables are projection machinery and are not in
`/v1/sql`'s public allow-list, so a value > 0 here is how an operator confirms
the incremental projection may scope a legacy fold at all. Read it after any
backfill of that layer, and after a full rebuild (which resets it).

**A Prometheus + Grafana server stays deliberately deferred** (team-lead
decision on issue 53): that is real operational weight — extra processes to run,
secure and resource-budget on an 8 GB box — against a single-process monolith
(ADR-0005). This endpoint is what makes standing one up a later, reversible
choice; until then the scrape is readable by hand or by any transient scraper.

Like the health probes, `/metrics` sits **outside the rate limiter** (a scraper
on a fixed cadence must not spend the public request budget) and, unlike them, is
**not** in the public CORS grant — it is an operator surface, so browser
JavaScript on another origin cannot read it.

### The off-box watcher (an external Claude scheduled routine)

The watcher must live **off the box** so it still fires when the app — or the
whole VPS — is down, and it must need no third-party account. The chosen shape is
an **external Claude scheduled routine** that polls `https://tenders.zebreus.click/health/deep`
**every 4 h** and alerts on any non-200 (or no response). Because `/health/deep`
folds uptime, ingest freshness, last-job outcome and disk into one 503-or-200
verdict, that single GET covers everything; the routine reads the JSON body to
name which check tripped, and can watch `disk.wal_bytes` for a mid-backfill WAL
runaway. This replaces the earlier plan of a hosted uptime monitor
(UptimeRobot/Better Stack) or a GitHub Actions cron — both were rejected together
with the storage box and a public GitHub repo (no external resources, 2026-07-21).

The 4 h cadence is a deliberate trade: cheap and account-free, at the cost of up
to ~4 h to notice an outage rather than the "within minutes" issue 24 first
aimed at. Acceptable for a single-operator, rebuildable dataset; tighten the
interval if that ever stops being true. The routine is configured outside this
repo, so nothing in the deploy ships it — it is an operator-owned schedule.

### Test procedure

- **Freshness / job / disk logic** is unit-tested against crafted signals
  (`crates/app/src/v1/health.rs` `#[cfg(test)]`) and end-to-end
  (`crates/app/tests/api.rs::the_deep_health_probe_reports_operational_health`:
  a fresh box is 200, a recorded `error` run flips it to 503).
- **The live alert path** (the acceptance drill, run *after* a deploy, in a
  quiet window with no backfill in flight): `systemctl stop tender-db` on the
  box, confirm the watcher routine alerts on its next poll (within ~4 h), then
  `systemctl start tender-db`. To exercise the freshness signal without waiting
  26 h, temporarily lower `INGEST_STALE_SECS`, deploy, and confirm
  `/health/deep` reports `ingest_freshness.ok = false` — then revert.

## TLS and nginx

- vhost: `/etc/nginx/sites-available/tenders.zebreus.click` (symlinked into
  `sites-enabled`; the stock `default` site is removed).
- cert: `/etc/letsencrypt/live/tenders.zebreus.click/{fullchain,privkey}.pem`,
  issued by `certbot --nginx`. Renewal is automatic via certbot's systemd timer
  (`systemctl list-timers 'certbot*'`); dry-run it with
  `certbot renew --dry-run`.
- The vhost sets `proxy_buffering off`, `proxy_cache off` and a 24 h read
  timeout so SSE streams flow unbuffered, and enables HTTP/2. Certbot manages
  the `listen 443 ssl` / cert lines; the `http2 on;` line is ours — re-check it
  after any certbot config rewrite.

```sh
nginx -t && systemctl reload nginx
```

## Disk watch

Two filesystems, watched separately:

- **`/data` — the 500 GB Hetzner volume**, the one that grows with ingestion.
  It carries both the raw archive and the database, because the parsed DB alone
  will not fit the 75 GB root disk (text satellites dominate — pilot-sizing.md).
  - `/data/archive/<source>/…` — raw fetched packages, immutable, append-only.
    TED under `ted/{daily,monthly}/`, DÖE under `doe/{daily,monthly}/`.
  - `/data/db/tender-db.db` (+ `-wal`) — the Turso database. The raw archive is
    the large static tenant (~178 GB and barely moving between backfills); the DB
    plus its WAL is what grows during a load.
- **`/` — the 75 GB root disk.** Pressure here is almost always the nix store
  (build artifacts + old bundles), not application data.

```sh
df -h /data /
du -sh /data/archive/* /data/db/*        # where the volume budget is going
ls -lh /data/db/tender-db.db-wal         # WAL size during a bulk load (issue 42)
```

A full backfill is the thing to plan for: the TED archive is the big one, and a
complete DÖE backfill is ~3 GB of ZIPs plus the projected rows. Watch `/data`
against a **~70 % operational guard** while a backfill runs — a self-imposed
ceiling well under the `/health/deep` 90 % alarm, leaving room to grow the volume
or pause the load before a write can fail mid-ingest. (The guard is a run-driver
convention, not a code constant; the only threshold in code is the 90 % deep-
health disk check.)

**WAL growth during bulk loads (issue 42).** Turso never auto-checkpoints fresh
frames, so a long `process`/`project` run would otherwise pile the whole run's
writes into `tender-db.db-wal` unbounded — it reached **13 GB and climbing**
during the first backfill. The processor now folds the WAL back at each package
boundary (a `wal_checkpoint(TRUNCATE)` at the writer-idle moment right after a
package commits), so the `-wal` stays bounded (single-digit GB) instead of
tracking the whole run. Idle pooled readers do not pin it; a reader mid-scan only
delays reclaim to the next package. The size is surfaced as `disk.wal_bytes` on
`/health/deep` — a large WAL mid-backfill is expected and never alarms, but a
*monotonically climbing* one across many packages is the signal that the
checkpoint is not reclaiming (investigate before `/data` fills).

Root-disk reclaim: `nix store gc` deletes unreferenced store paths — **including
old bundles you might want for a rollback**, so switch the symlink to the bundle
you want to keep before running it, or verify the current one is safe.

Note that nothing outside the service writes to `/data/db`: turso is
single-process, so the running server holds the database open exclusively (see
the Ingestion rule above). A scratch `*.db` may appear here from earlier
dev/verification work — harmless, but never point a CLI at the production file.

## Disaster recovery (there are no backups)

**The snapshot feature and its local ring were removed on 2026-08-06** (owner
decision under storage pressure; commits 39c0e08/aa9f2a1/1faf9d9). Since then
there is **no copy of the DB anywhere**, on-box or off. DR = re-ingest from
sources. The full scenario analysis, measured stage rates, and the
recommendation menu for re-introducing a minimal backup live in
`docs/research/dr-premise-2026-08.md` (2026-08-09); the honest numbers:

- **Canonical layer damaged** (bad projection, layer wipe; DB file healthy):
  `project rebuild=true` + verify ≈ **1–1.5 days**. The public instance serves
  an empty/partial tender layer the whole time (ADR-0009).
- **DB file lost** (archive intact): ≈ **4–6 days** — and today that includes a
  forced full ~180 GB re-download, because the `fetches` registry (which
  `process` walks) is itself DB-resident; there is no register-existing-files
  path yet (dr-premise §2 — a cheap planned fix).
- **Box lost, volume survives**: ≈ **0.5–1 day** (re-provision per this doc;
  everything on the root disk is regenerable).
- **NOT recoverable at any cost**: `users`, `api_tokens`, `webhook_endpoints`,
  and change-cursor/epoch continuity (< 1 MB today). No email exists on
  accounts by design, so account loss is permanent per user. An off-box copy
  of exactly this state is the standing pre-launch recommendation
  (dr-premise §7(b)), pending the owner's re-decision.

Mechanism knowledge worth keeping (if a backup path returns): turso has no
online-backup API and `VACUUM INTO` OOMs at scale (turso-scale.md §1); the
workable mechanism is writer-held `wal_checkpoint(TRUNCATE)` + file copy +
offline verify by `integrity_check` **plus row-count comparison** (a killed
copy can pass integrity_check alone). Measured at 455 GB on this box:
**941 s writer freeze + 606 s verify** (job 570, 2026-08-06 — the last
snapshot ever taken). Restore was a plain file copy: stop service, swap
`.db`, delete `-wal`/`-shm`, chown, start, `/health`.

### Disk headroom

`/data` (1 TB since 2026-07-22) holds archive (~180 GB) + DB (560 GB and
growing; it can never shrink — VACUUM is impossible). 219 GB free as of
2026-08-09; a plain on-box DB copy no longer fits (XFS reflink copies do).
Growth model and volume-full forecast: `docs/research/` storage-lifecycle
study (issue 169).


## Verification

Two black-box binaries check a *running* instance from the outside — they talk
only to its public API (default `https://tenders.zebreus.click`), never to the DB
or the box, so they run from the dev machine. Both use the account-gated
read-only `/v1/sql` endpoint for the counting queries and need an API token
(`--token`, or `TENDER_API_TOKEN`); without one the token-gated checks are
reported *skipped*, never silently passed.

- **`verify`** (`crates/ingest/src/bin/verify.rs`) — the standing **acceptance
  harness**: pass/fail against *external* ground truth. It checks per-year TED
  coverage against the vendored counts, cross-checks a few eForms days against the
  live TED Search API (set membership), and walks one real notice per format era
  through the API (Notice → Tender → award → winner). Exits non-zero on any
  executed check that fails or could not run. **Run it after a deploy and after a
  backfill** to confirm the instance still meets ground truth. A partially
  backfilled instance honestly reports failure for the years it does not yet hold.
- **`data-quality`** (`crates/ingest/src/bin/data-quality.rs`) — the descriptive
  sibling: **no pass/fail**, it *measures* how complete the imported data is
  (per-era field completeness, award linkage, results materialisation, TED↔DÖE
  merge) with bounded `GROUP BY` aggregates, safe against the live rate-limited
  SQL endpoint. **Run it to read the numbers** after a parser change or backfill,
  when you want more depth than the dashboard's data-quality panel.

```sh
# From a dev checkout (nix provides the toolchain); --json for machine output.
TENDER_API_TOKEN=<token> cargo run -p ingest --bin verify
TENDER_API_TOKEN=<token> cargo run -p ingest --bin data-quality
```

### Running the tests

`crates/app` is a Dioxus fullstack crate, so every server-side module — the `v1` API,
the supervisor, health, metrics — sits behind `#[cfg(feature = "server")]`; the same
crate also builds to WASM for the browser client. A plain

```sh
cargo test -p tender-db      # DON'T: compiles none of the server modules
```

finds none of those 83 unit tests and prints `test result: ok. 0 passed; 0 failed`,
which reads like a pass and checked nothing. Use the workspace alias:

```sh
cargo test-app               # the app crate's server-side unit tests
cargo test-app health        # filtered, as usual
```

`store`, `ingest` and `model` have no feature gates, so `cargo test -p store` and
friends already run everything. On a tight disk, prefix any of these with
`CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0`: debuginfo for this
workspace's test binaries runs to tens of gigabytes and nothing here needs it.

## Open items

Known gaps in the production setup, tracked here so they aren't rediscovered:

- **Reboot survival is unexercised.** The unit is `enabled` (survives reboot by
  configuration), but no actual reboot has been done to confirm the service, the
  `/data` mount, and nginx all come back clean. Needs a deliberate quiet window —
  do it when no ingestion/backfill is in flight, then verify `systemctl status
  tender-db` and `curl https://tenders.zebreus.click/health`.
- **The off-box watcher is a per-4h Claude routine, not sub-minute.** `/health/deep`
  ships and covers uptime, freshness, disk and job failures, and the external
  Claude scheduled routine polls it every ~4 h (see
  [Monitoring and alerting](#monitoring-and-alerting)). That means an outage can
  go unnoticed for up to ~4 h — accepted for now given the rebuildable dataset
  and single operator; tighten the interval if that changes.
- **AGPL source offer is a written offer, not a public repo.** `/_source`
  currently tells a network user to request the Corresponding Source from the
  operator (AGPL §13 permits this). A public GitHub repo was declined for now (no
  external resources, 2026-07-21), so the written offer stands.
