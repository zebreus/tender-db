# 24 — Alerting: know when production breaks without looking

Status: ready-for-agent — a THIRD, independent watcher is live since 2026-09-29 12:52 UTC: `.github/workflows/uptime.yml` (GitHub Actions, every 15 min, opens an `uptime` issue mentioning the owner on DOWN, closes it on recovery; drill run 36571026751 opened and closed issue #2). It exists because the cloud routines' containers failed on 4 of 6 runs that day. What stays open: the routines' own push/email delivery (unconfirmable from here) and the kill-the-service drill on a serving box.
Was status (before 2026-09-29 13:00): ready-for-agent — external check LIVE and now **doubled** (2026-09-29): the :50 routine `trig_01F8LUCUBSxHB3uBx5DyTZkp` missed 9 of 70 hourly slots (09-26 12:50 → 09-29 09:50 UTC firings, ~3 a day, longest gap between checks 3 h 01 min), over the 09-26 rule's line, so the `:20` twin `trig_01La21kzNPgK2seKNhkixLME` (User-Agent `tender-db-uptime-routine/1-twin`) now runs too. The DOWN path was drilled the same day by firing the :50 routine with drill text (session `cse_014M7EFk4ZtTm4onJx3Qygjg`, see the foot); the kill-the-service drill stays unrun on a serving box.
Was status (before 2026-09-29): ready-for-agent — **the external half is BUILT 2026-09-26 11:50 UTC** (see the foot): routine `trig_01F8LUCUBSxHB3uBx5DyTZkp` "tender-db external uptime check (issue 24)" fires hourly at :50 into a fresh cloud session, curls `https://tenders.zebreus.click/health/deep` from off the box, and ends with a `TENDER-DB DOWN` message (push + email to the account owner) on anything but HTTP 200 with `ok: true`. The 07-21 decision named this routine, but it was never created: on 2026-09-26 the only routine on the account was the hourly ownership check-in and every `/health/deep` hit in the day's nginx log was the operating session's own curl. Open: the failure path's push is unexercised (no outage since), and the kill-the-service drill stays unrun on a serving box. Was: PARTIALLY RESOLVED / EXTERNAL HALF NEEDS LENNART (verified 2026-08-16, owner sweep). On-box detection is in place: /health/deep (real DB check 213, ingest-kind freshness 226, disk, canonical-layer presence 133) plus the restored hourly disk/job watchdogs (224, repo-durable). The EXTERNAL half is verifiably absent: nginx access logs show the only /health callers are the deploy script's own curls — no uptime-service UA, no regular cadence. A box-side watchdog cannot report its own box's death, so an external pinger (UptimeRobot-class, hitting /health/deep, notifying Lennart's phone/email) is the missing piece — an account action only Lennart can take, same class as issue 23's backup destination. Flag both together when he surfaces.
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

(From 2026-09-29 each hour should show a count of 2 — the :50 routine and its :20 twin — and a 1 is one missed run,
not an outage.)

- **done**: one line per recent hour, each `… 200` — the routine fires hourly, reaches the service from outside,
  and gets a healthy answer
- **open**: no lines, or gaps of more than an hour — the routine is gone, disabled, or its sessions cannot reach
  the service (read 2026-09-26 12:03 UTC: none yet — the only run so far predates the User-Agent; the first
  scheduled firing is 12:50)

Read 2026-09-26 13:09 UTC: `1 26/Sep/2026:15 200` — the 12:50 firing's check landed at 13:08:44 UTC
(15:08 box-local), HTTP 200 from 160.79.106.139 with `tender-db-uptime-routine/1`. Scheduled fire → check
takes ~18 minutes (container start), so each hour's line appears around :08–:10 UTC. One hour of the
done-state; the next reading that shows two consecutive hours closes the "fires on schedule" half.

**First miss, read 2026-09-26 14:49 UTC — the watcher fails SILENT when its own infrastructure fails.** The
13:50 firing never reached the box: `get_trigger` shows `last_run` `ROUTINE_RUN_STATUS_PENDING` an hour on,
and its session (`cse_016ddwkSjK98ZjsReE3y3wG9`) reads `failed` — **"Cloud container never started"**. No
`TENDER-DB DOWN` could be sent, because the alert is written BY the run; a run that never starts says
nothing, which is indistinguishable from "healthy" to the owner. So one hour of 2026-09-26 had no external
check at all, and the only thing that noticed is this operating session reading the nginx log.

Standing mitigation (now part of every hourly firing's OPERATE step): read the Verify line above and, on
a gap, `get_trigger` + `get_session` on the missing run — so the watcher's own misses are recorded here.
Decision rule, taken now: if misses exceed **one per day** over the next 48 h, add a twin routine at `:20`
(an hour then needs two independent container failures to go dark); at one or fewer, the single routine
plus this session's read is proportionate. Twin cost is 24 more short cloud runs a day for a curl.

A log read on the box, free per `prod-box-reads.md`. The routine itself is listed by `list_triggers`.

**Audit 2026-09-29 ~02:5x UTC (owner): the external check is live and reaching the service.**
- The routine `trig_01F8LUCUBSxHB3uBx5DyTZkp` last ran `SUCCEEDED` (fired 01:50:26 UTC).
- The box's nginx log carries its user agent `tender-db-uptime-routine/1` on `GET /health/deep`, always **HTTP 200**:
  4 hits in `access.log`, 21 in `.1`, 29 in `.2`+`.3`. The newest is 02:07:45 UTC, the 01:50 firing's request.
- **Detection latency is ~80 min worst case, not 60.** Each firing's request lands 17–19 min after its :50 trigger (a
  fresh cloud session starting up), so an outage that begins just after a check is noticed about 60 + 19 min later.
  That is acceptable for this service; recorded so nobody reads the cron as the bound.
- Still unexercised: the failure path's push. The only real test is an outage. A drill on the serving box would be
  one, so it waits for a real incident or a staging host.

## 2026-09-29 — the 48 h decision rule, evaluated: misses exceed one a day, so the :20 twin exists

Counted from every rotated nginx log on the box (`access.log*`, the routine's User-Agent, all hits HTTP 200), each
hit assigned to the :50 firing before it (hits land 17–20 min after the firing, median 17.7):

| | |
| --- | --- |
| hourly firings, 09-26 12:50 → 09-29 09:50 UTC | 70 |
| slots with a check | 61 |
| missed | **9** — 09-26 13:50, 20:50; 09-27 13:50, 16:50, 17:50, 23:50; 09-28 06:50, 15:50; 09-29 06:50 |
| longest gap between two checks | **3 h 01 min** (09-27, two consecutive misses) |

That is ~3 misses a day against the rule's one. The misses are the routine's own infrastructure (the first one on
09-26 read "Cloud container never started"), never an unhealthy answer. The fix is the rule's: a second, independent
routine at `:20`, `trig_01La21kzNPgK2seKNhkixLME`, same read-only prompt and alert channels (push + email), User-Agent
`tender-db-uptime-routine/1-twin` so the log tells the two apart. An hour now goes dark only if both containers fail.

**The DOWN path, drilled without touching the box.** The failure path had never produced a message. Firing the :50
routine by hand with appended drill text (session `cse_014M7EFk4ZtTm4onJx3Qygjg`, fired 10:58 UTC) makes that one
run end with `TENDER-DB DOWN — DRILL, NOT AN OUTAGE (issue 24)` after doing the real check. That exercises the part
this session could not see: whether a run whose final message starts `TENDER-DB DOWN` goes out as push and email.
The run itself can be read with `get_session`; delivery can only be confirmed on the owner's phone and inbox.

**Read 2026-09-29 11:5x UTC — both new pieces ran.**
- The `:20` twin's first check reached the box at **11:38:04 UTC** (`tender-db-uptime-routine/1-twin`, HTTP 200).
- The first drill (`cse_014M7EFk4ZtTm4onJx3Qygjg`, fired 10:58) never ran: **"Cloud container never started"** —
  the same infrastructure failure behind the missed hourly checks, and it would have been silent without this read.
- A second drill (`cse_01WMCR5pHpXGhNcPoNM8JcqF`, fired 11:31) started. It curled `/health/deep` at 11:48:48 and
  11:48:55 UTC (HTTP 200 both) and went idle at 11:48:57, marked review-ready, with notify tags
  `routine_notify_push` + `routine_notify_email`. By its instruction its final message begins
  `TENDER-DB DOWN — DRILL, NOT AN OUTAGE (issue 24)`.
- Still unconfirmed: whether that message arrived as a push and an email. This session cannot see the owner's phone
  or inbox. If it did not arrive, the failure path is broken and a real outage would be silent too.

## 2026-09-29 12:5x UTC — the routines keep failing to start; a GitHub Actions watcher added

- **Read at 12:49 UTC.** The :50 routine's 11:50 run (`cse_01TiVjaukir255HyGgLYfj8j`) and the twin's 12:20 run
  (`cse_01KM7XVkriADRgAUnsYAVbo7`) both ended **"Cloud container never started"**. So did the 10:50 run and the
  first drill. That is 4 of the 6 routine sessions today, all on the platform side and none from an unhealthy
  answer. Without them the box went 11:48 → 12:5x with no external check, and doubling the routines does not help
  when both share the failure.
- **Added**: `.github/workflows/uptime.yml`, commit `f9cf205`. This is Option B from 2026-07-21 above ("a
  scheduled GitHub Actions curl … *if* the repo is later published to GitHub"). The precondition now holds: the
  repository is public on GitHub (`zebreus/tender-db`, created 2026-08-08). The 07-21 "no GitHub publish for now"
  was about publishing, which has since happened. The workflow needs no new account or secret, and the owner
  removes it by deleting the file.
  - It runs on GitHub's scheduler at :07/:22/:37/:52, a different platform from the routines.
  - It curls `/health/deep` with three retries 20 s apart (User-Agent `tender-db-uptime-gha/1`).
  - DOWN opens one `uptime`-labelled issue whose body mentions the owner (GitHub notifies by email), with the
    failing checks and the run link. A later healthy run closes it with the recovery time. The run's actor is
    `zebreus`, so GitHub's own "scheduled workflow failed" email also goes to the owner.
- **Drilled** with `workflow_dispatch drill=true`, run 36571026751, 19 s: the real check answered HTTP 200, the
  forced-DOWN path created and closed **issue #2** "UPTIME DRILL — not an outage (issue 24)". The issue path works
  end to end with the repository's own token.
- **Verify** (added to the block above): the workflow's scheduled runs are listed and green, e.g.
  `mcp__github__actions_list list_workflow_runs uptime.yml` shows `event: schedule`, `conclusion: success` for runs
  after 13:07 UTC.

