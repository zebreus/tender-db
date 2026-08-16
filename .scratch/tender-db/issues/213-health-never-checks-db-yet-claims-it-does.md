# 213 — /health can never report unhealthy but claims "the database answers"; the 503 branch is dead code

Status: RESOLVED — DEPLOYED & VERIFIED 2026-08-16 (serving rev `005c617`). Prod re-probe: `/health` now
returns `{"cursor":…,"ok":true,"rev":…}` with **no `database` field** (honest liveness), and `/health/deep`
still carries its `database` check (now the real reader-pool read). Fix in `67c947d` ("health: make /health honest liveness
+ give /health/deep a real DB-answer check"), pushed to `origin/main` + the handover branch. Owner chose a
blend of both options: **shallow `/health` → Option 2** (honest liveness — keeps issue-61's instant,
DB-free design so a saturated reader pool during a projection never fails the deploy gate; dropped the dead
`is_some()`/`unavailable`/`503` scaffolding and the false `database` field; reworded handler doc, OpenAPI,
`/docs`, operations.md); **deep `/health/deep` → Option 1 where it belongs** (its `database` verdict now
comes from the real reader-pool `recent_job_runs` read it already makes — WAL-served, never blocks the
writer, so issue-61-safe — instead of the infallible in-memory cursor; `assess()`'s previously-unreachable
unhealthy branch is now live). Rejected a `SELECT 1` on shallow `/health`: it would flap to 503 during
heavy folds when the reader pool is busy, defeating the reason `/health` is DB-free.

Tests green: `assess()` unit suite (10) incl. `cursor:None → unhealthy`; api integration
`the_service_root_and_health_answer` (now asserts `/health` carries no `database` field),
`the_deep_health_probe_reports_operational_health`, `the_openapi_spec_matches_the_served_surface`.

**NOT yet deployed** — same harness deploy gate blocking issue 219 (`./deploy.sh`/`git push vps` refused
this session; flagged to Lennart). Prod still serves `8938e02`. Response-shape note: `/health` no longer
returns the always-`"ok"` `database` field — monitors keying on HTTP status are unaffected; any keying on
that field should move to `/health/deep`.

Was: needs-triage — CONFIRMED (code) 2026-08-15. Filed from the API review (subagent).
Kind: correctness / honesty (the deploy readiness probe)
Blocked by: —
Relates to: 61 (the deliberate no-DB, instant-`/health` design this half-implements), 161 (health-signal
freshness), 205 (the boot outage a real DB probe would have surfaced)

## Symptom

`/health` always returns `200 {"ok":true,"database":"ok"}`. It performs no database access, so a database
that has stopped answering after boot still reads healthy — and `deploy.sh`, which greps `ok:true`, gates
on a check that cannot fail. Its own doc comment and the published docs say the opposite.

## Root cause

`crates/app/src/v1/mod.rs:864-876`:

```rust
let cursor = Some(state.db.current_cursor());   // :867 — unconditionally Some
… "ok": cursor.is_some(),                        // always true
  "database": if cursor.is_some() { "ok" } else { "unavailable" },   // always "ok"
let status = if cursor.is_some() { OK } else { SERVICE_UNAVAILABLE }; // always OK
```

`current_cursor()` is an in-memory watch read (`crates/store/src/lib.rs:748-750`,
`*self.cursor.borrow()`), infallible and never `None`. So the `"unavailable"` / `503` branches are
unreachable. The comment at `:865-866` is honest ("no DB access, so the … probe stays instant") — but the
handler doc at `:862-863` ("the database answers"), `openapi.json:563` ("Process up and database
answering") and `docs.rs:416` ("database answers") all claim a check that does not happen.

(`/health/deep`'s own `database` sub-check has the same vacuous `cursor.is_some()` shape at
`health.rs:57`/`115`; its ingest/job/layer checks do hit the DB, so deep is only partly affected.)

## Failure scenario

The reader pool / DB file becomes unreadable post-startup. `GET /health` still returns
`200 {"ok":true,"database":"ok"}`; monitors and the deploy gate that trust the documented "database
answering" semantics are misled into treating a broken box as ready.

## Fix (pick one — this is a small change either way)

1. **Make the claim true**: do one O(1) DB touch (e.g. `SELECT 1` on a borrowed reader, short-timeout) so
   `database` reflects reality; keep the `unavailable`/503 branch, now reachable.
2. **Make the docs true** (the issue-61 intent — keep `/health` instant): reword the handler doc,
   OpenAPI, and `/docs` to "process-liveness probe; does not verify the database — see `/health/deep`,"
   and delete the dead `is_some()`/`unavailable`/503 scaffolding so the code stops implying a check it
   never runs. If deploy readiness needs a real DB signal, point `deploy.sh` at `/health/deep`.

Option 2 preserves the instant-probe design; option 1 restores the stronger guarantee the text promises.
Team-lead call on which contract `/health` should carry.

## Verification

- Option 1: with the DB made unreadable in a test harness, `/health` → 503. Deploy gate still passes on a
  healthy box.
- Option 2: no code path claims a DB check; grep of `openapi.json`/`docs.rs`/handler doc finds no
  "database answers" wording on `/health`.
