# 24 — Alerting: know when production breaks without looking

Status: ready-for-agent — **the external half is BUILT 2026-09-26 11:50 UTC** (see the foot): routine `trig_01F8LUCUBSxHB3uBx5DyTZkp` "tender-db external uptime check (issue 24)" fires hourly at :50 into a fresh cloud session, curls `https://tenders.zebreus.click/health/deep` from off the box, and ends with a `TENDER-DB DOWN` message (push + email to the account owner) on anything but HTTP 200 with `ok: true`. The 07-21 decision named this routine, but it was never created: on 2026-09-26 the only routine on the account was the hourly ownership check-in and every `/health/deep` hit in the day's nginx log was the operating session's own curl. Open: the failure path's push is unexercised (no outage since), and the kill-the-service drill stays unrun on a serving box. Was: PARTIALLY RESOLVED / EXTERNAL HALF NEEDS LENNART (verified 2026-08-16, owner sweep). On-box detection is in place: /health/deep (real DB check 213, ingest-kind freshness 226, disk, canonical-layer presence 133) plus the restored hourly disk/job watchdogs (224, repo-durable). The EXTERNAL half is verifiably absent: nginx access logs show the only /health callers are the deploy script's own curls — no uptime-service UA, no regular cadence. A box-side watchdog cannot report its own box's death, so an external pinger (UptimeRobot-class, hitting /health/deep, notifying Lennart's phone/email) is the missing piece — an account action only Lennart can take, same class as issue 23's backup destination. Flag both together when he surfaces.
Current monitoring is tmux loggers writing files on the box — nobody is
notified if the service dies, /health goes red, disk fills, or the daily
continuous-mode jobs stop landing. The /v1 view-staleness 500s went
unnoticed until manually observed; continuous operation (issue 15
acceptance: 3 days of live updates) needs eyes that don't sleep.

Scope (keep minimal — this is a one-operator project, not an SRE stack):
- External uptime check on https://tenders.zebreus.click/health with
  email/push notification (a free hosted pinger is fine and is the one
  piece that must NOT live in the app, since it must fire when the app
  is down).
- In-app health surface for the pinger to judge: /health already exists;
  extend it (or a /health/deep) to go unhealthy when the last successful
  scheduled ingest is older than expected (TED > ~26h) or disk usage
  crosses a threshold — then the single external check covers freshness
  and disk too.
- Alert on job failures: supervisor marks a job ERRORED → surfaced in
  the same health signal.

Acceptance: kill the service → notification arrives within minutes;
simulate stale ingest (clock the threshold) → health flips unhealthy;
documented in the runbook.

## Comments

### Implementation (needs-verification)

**Design — separate `/health/deep`, not a widened `/health`.** `/health` stays
the fast liveness probe `deploy.sh` greps for `ok:true` (its contract): its `ok`
must reflect *only* liveness, or a deploy would be failed by a stale-ingest or
full-disk condition that has nothing to do with the new build being live. A new
`GET /health/deep` (`crates/app/src/v1/health.rs`) carries the operational
signals. This keeps the two concerns from fighting and leaves the deploy check
untouched.

**What `/health/deep` covers** — one verdict (`ok`), HTTP **200** when all pass /
**503** when any fails, so a plain hosted pinger judges it by status alone; the
body names the tripped check:
- **database** — the same reader-pool cursor read `/health` does.
- **ingest_freshness** — 503 when no `job_log` run has succeeded in 26h
  (`INGEST_STALE_SECS`). A never-run box is healthy-but-unmeasured, not alarmed.
- **last_job** — 503 when the newest finished run's `outcome = "error"` (a
  Supervisor ERRORED job); self-clears on the next success.
- **disk** — 503 once ≥90% (`DISK_FULL_FRACTION`) of the `TENDER_DB` volume
  (`/data`) is used, via `fs4::statvfs`. Unmeasurable → healthy-but-unmeasured.

All reads go through the store **reader pool** (`state.db.recent_job_runs`,
`state.readers`), never the writer (issue 20). No edits to `supervisor.rs` or
the store crates — freshness/errored are derived from the existing `job_log`, so
this doesn't collide with issues 21/23.

**External pinger — needs a user decision (can't ship in-repo).** The repo's
only git remote is the VPS bare repo, **not GitHub**, so there is nowhere
credential-free in the codebase to run a scheduled check. The pinger must be an
account the operator creates off-box. Documented in `docs/operations.md`
(Monitoring and alerting): **Option A** a hosted monitor (UptimeRobot / Better
Stack, free tier, exact setup steps) — recommended, works today; **Option B** a
scheduled GitHub Actions curl (credential-free via GitHub's auto-email on a
failed run) *if* the repo is later published to GitHub. Tracked as an open item.

**Tests (green).** Unit: `crates/app/src/v1/health.rs` `#[cfg(test)]` — 6 tests
over crafted signals (db down, stale, boundary, never-run, errored, full/
unmeasured disk). E2E: `crates/app/tests/api.rs::the_deep_health_probe_reports_operational_health`
— fresh box → 200; a recorded `error` run → 503. Clippy clean. Verified in an
isolated HEAD worktree (the shared worktree was transiently broken by a
concurrent agent's supervisor.rs WIP).

**Remaining (post-deploy).** The kill-the-service acceptance drill and the
stale-threshold check happen after this deploys and the pinger account exists —
procedure in the runbook's "Test procedure".

### 2026-07-21 — External pinger: Claude scheduled routine (team lead)

Lennart: no third-party pinger accounts / no GitHub publish for now.
Decision: the off-box check runs as a Claude scheduled cloud routine
(curls https://tenders.zebreus.click/health/deep, notifies Lennart on
non-200) — off the VPS as required, no new accounts. Set up by the lead;
the in-app /health/deep half is deployed (bad8dda).

### 2026-09-26 — the off-box check exists now

**What was missing.** The 2026-07-21 decision above chose a Claude scheduled cloud routine as the pinger
and said it was "set up by the lead". It was not there on 2026-09-26: `list_triggers` returned one routine
(the hourly ownership check-in, which runs *in* this operating session and reads health over ssh on the
box), and the day's nginx log held eight `/health` callers, seven `curl/8.5.0` from the operating
session's own checks and one crawler. Nothing outside the box was watching.

**What was built.** Routine `trig_01F8LUCUBSxHB3uBx5DyTZkp`, "tender-db external uptime check (issue
24)": cron `50 * * * *` (the server anchored the hourly `0 * * * *` to the creation minute), a FRESH
session per firing (so it does not depend on the operating session being alive), notifications push +
email. Its prompt is read-only — one `curl -sS --max-time 30 --retry 2 --retry-delay 20
--retry-all-errors https://tenders.zebreus.click/health/deep`, then a verdict: `tender-db healthy, rev
…` when HTTP 200 and `ok: true`, otherwise a message starting `TENDER-DB DOWN` with the status or curl
error, the time, and every failing check's detail from the body. The retries mean a single dropped
request is not an alert; a 503 (a check failing), a timeout, a TLS or connection error, or `ok: false`
is. It holds no connectors and needs none. Fired once by hand at creation (session
`cse_01GFnBXP1dwyjBdeWVEcNoB2`) to verify the healthy path from outside.

**Still open.** The failure path's delivery (does a `TENDER-DB DOWN` run actually push?) is unexercised,
and the runbook's kill-the-service drill is not being run against the serving box for it. The first real
outage, or a planned maintenance restart, is the drill: its firing should push. The Verify block below
reads the routine's existence and cadence.

**First run, read 2026-09-26 12:03 UTC.** The hand-fired session (`cse_01GFnBXP1dwyjBdeWVEcNoB2`) was
created at 11:50:28, its container took ~12 minutes to start, and its request reached nginx at 12:02:33
(`200`, 608 bytes); the session went idle at 12:02:36 having done exactly that (266 output tokens, ~$0.20 a run,
so ~$150 a month at hourly — accepted, per the standing "token cost is not a constraint"). Two consequences:
an alert lands up to ~15 minutes after the :50 firing, and a minute window cannot identify the routine's hits.
So the prompt now sets `-A 'tender-db-uptime-routine/1'`, and the Verify block counts that User-Agent. (The
operating session's own curls come from the same `160.79.106.x` range, so neither address nor minute tells
them apart.)

## Verify

    ssh -o BatchMode=yes root@zebreus.click "grep -h 'tender-db-uptime-routine' /var/log/nginx/access.log | awk '{print substr(\$4,2,14), \$9}' | uniq -c | tail -4"

- **done**: one line per recent hour, each `… 200` — the routine fires hourly, reaches the service from outside,
  and gets a healthy answer
- **open**: no lines, or gaps of more than an hour — the routine is gone, disabled, or its sessions cannot reach
  the service (read 2026-09-26 12:03 UTC: none yet — the only run so far predates the User-Agent; the first
  scheduled firing is 12:50)

A log read on the box, free per `prod-box-reads.md`. The routine itself is listed by `list_triggers`.
