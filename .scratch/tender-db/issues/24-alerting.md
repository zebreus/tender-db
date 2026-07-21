# 24 — Alerting: know when production breaks without looking

Status: needs-verification

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
