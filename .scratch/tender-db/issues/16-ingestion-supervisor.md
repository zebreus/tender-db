# 16 — In-app ingestion supervisor, admin API, progress panel

Status: resolved
Blocked by: 06

Goal: ingestion runs inside the server process (ADR-0005 made real): no
external process ever touches the production DB, no downtime, and operations
are triggered/observed via authenticated endpoints + the dashboard.

Context: until now fetch/process/project were standalone CLIs; a production
load required stopping the service (turso is single-process). That pattern is
retired for production use — CLIs remain dev tools for scratch DBs only.

Scope:
- `ingest`: a Supervisor task (tokio) owned by the app at startup: a small
  job queue (fetch-source-period, process-pending, project, scheduled-tick)
  executed sequentially (one job at a time — the writer is single anyway),
  with per-job progress state (job kind, current package, done/total members,
  notices/s, started_at) held in shared state and a bounded recent-job log
  (outcome, duration, counts) persisted to the store.
- Scheduler: TED daily probe after 09:35 CET Mon–Fri (+ finality re-fetch of
  the current day), DÖE completed-day fetch daily, process+project after any
  fetch. Backfill jobs (a source + period range) enqueue the same way.
- `app`: `/admin` axum routes gated by a preshared operator secret
  (TENDER_ADMIN_SECRET env; constant-time compare; 404 when unset):
  POST /admin/jobs (enqueue fetch/backfill/process/project/reprocess-
  quarantine), GET /admin/jobs (queue + progress + recent log), DELETE
  /admin/jobs/{id} (cancel pending). JSON, curl-friendly.
- Dashboard: an Ingestion panel showing the supervisor state live (current
  job + progress bar, queue, recent runs with outcomes); admin actions stay
  API-only (the dashboard is public — no secret in the browser).
- The server binary must run ingestion with release-grade throughput —
  document in docs/operations.md that production ingest happens ONLY via the
  supervisor (the deployed binary is a release build by construction).
- Tests: supervisor unit tests with a fake job (progress transitions,
  sequential execution, cancel); admin-auth tests (secret required,
  constant-time, unset ⇒ 404); integration: enqueue process job over a
  fixture archive → progress observable → counts land.

Acceptance: on production, POST /admin/jobs with the secret ingests a
pending package with the service serving throughout (no 502s); the dashboard
shows the run live; the scheduler picks up the next TED daily without any
operator action.

## Answer

Implemented in 8636abb (supervisor + /admin + dashboard panel + scheduler),
live in production via rev 7199300. First supervisor-driven load executed by
the lead through /admin with the service serving throughout (zero downtime):
3594 Tenders / 3715 versions / 12478 Lots / 8221 Organizations / cursor
24553 from the real TED daily 2026-00136. Quarantine honest at 28 (21
nested-monthly tarballs pending walker recursion — assigned to the DOE/walker
slice — and 7 unrepresentable values). Admin gating verified live (403 paths).
The concurrent-deploy race observed during rollout is fixed in deploy.sh
(flock + regression refusal, f8bed0b).
