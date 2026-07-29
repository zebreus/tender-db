# 69 — DÖE has no fetch walk-forward (missed tick = silent permanent hole) + admin "run daily now" endpoint

Status: in-review
Kind: correctness (completeness) + ops
Blocked by: —
Surfaced by: the 2026-07-28 issue-61 incident recovery (snapshot restore + manual DÖE backfill)
Relates to: CONTEXT.md "nothing silently dropped" completeness promise; ADR-0004

## Two linked gaps this incident exposed

After the 07-27 snapshot restore, the daily 07-28 scheduler tick had been missed
during the incident. Recovery showed:

### (a) DÖE fetch has NO walk-forward — a missed tick is a silent permanent hole

TED's daily path PROBES forward from the DB's last-known fetch state and pulls
everything missing, so a skipped day self-heals at the next tick. **DÖE
(oeffentlichevergabe.de) does NOT** — it fetches a single "yesterday" package per
run, no catch-up walk. So the missed 07-28 tick left a real **doe 07-27 hole** that
would NEVER self-heal on its own; it had to be closed by a manual admin
fetch→process→project (fetch doe 07-27 → 93 notices → 92 tenders). Left unnoticed,
that day's German notices would be permanently absent — a silent violation of the
"nothing the source era publishes is silently dropped" promise (CONTEXT.md / ADR-0004).

**Fix direction:** give DÖE the same walk-forward discipline as TED — on each daily
run (or on startup), fetch every missing day from the last successful DÖE fetch
watermark up to yesterday, not just a single "yesterday". Bounded (DÖE is T+1,
so the catch-up set is small) and idempotent (hash-based fetch dedup already
handles re-fetch). This closes the "missed tick = permanent hole" class for the
German source.

### (b) No admin "run daily now" endpoint — post-downtime catch-up waits ~19h

The daily reconciliation (probe → process → project) is only reachable via the
scheduler tick (09:35 Berlin / 07:35 UTC). The admin enqueue API exposes
fetch/process/project/backfill/snapshot individually but NOT the composite daily
job (`enqueue_daily` / the probe). So after downtime (like this incident) there is
no clean one-shot "catch up now" — you either wait for the next tick (~19h) or
hand-run fetch→process→project per source (what we did). 

**Fix direction:** expose `enqueue_daily` (the probe-forward composite) via the
admin jobs API — `POST /admin/jobs {"kind":"daily"}` or similar — so an operator can
force the full daily reconciliation immediately after downtime instead of waiting
for the scheduler. Small surface (the composite already exists for the scheduler;
just wire an enqueue path).

## Scope / priority

- (a) is the correctness one (completeness promise) — a missed DÖE day is a real
  hole. Prioritize the walk-forward.
- (b) is ops convenience but was directly painful in this incident (manual per-source
  backfill under time pressure). Cheap to add alongside.
- Neither is urgent NOW: this incident's DÖE hole is already backfilled, and the TED
  suffix self-heals at the next tick. This issue is to prevent the NEXT missed tick
  from silently dropping DÖE data, and to make post-downtime catch-up a one-liner.

## Implementation (in-review, awaiting team-lead deploy)

Both parts done on branch `issue62-defer-org-indexes`:

- **(a) DÖE walk-forward** — new `fetch::probe_doe_daily` (crates/ingest/src/fetch.rs)
  mirrors `probe_ted_daily`: from the newest DÖE daily period already registered
  (`latest_doe_day` via `MAX(period)`), walk forward one calendar day at a time up
  to and including `end` (yesterday UTC, the freshest completed T+1 day), fetching
  each. A normal run advances one day; a gap catches up every missed day. Starts at
  watermark+1 (not at it) — DÖE completed days are final, so no TED-style finality
  re-check; and with no DÖE daily on record it fetches only `end` (never a full
  backfill). The scheduler's `enqueue_daily` now enqueues a `Spec::ProbeDoe` job in
  place of the single-yesterday `Spec::Fetch`. The `refetch` param was dropped as
  dead weight (starting past the watermark, it can never re-touch a registered day).
- **(b) run-daily-now** — `POST /admin/jobs {"kind":"daily"}` routes to
  `enqueue_daily(true)` (crates/app/src/supervisor.rs `enqueue_request`), forcing the
  full daily reconciliation (both source probes → process → project → snapshot) now.
  `enqueue_daily` now returns the enqueued job ids so the admin response reports them.
- **Tests** (crates/ingest/tests/fetch.rs): `doe_walk_forward_catches_up_a_multi_day_gap`
  (seed watermark → 3-day gap catch-up → idempotent no-op re-run) and
  `doe_walk_forward_crosses_month_boundary`. Gate green: full ingest fetch suite +
  store/app suites (41 lib tests incl. supervisor) pass; new code is clippy-clean.
