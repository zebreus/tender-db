# Capacity model v0 — what is known, and the campaign design (issue 167)

2026-08-09. Research gap #1 (research-gaps-2026-08.md) split into its two
halves: (A) everything already measured about the public surface's cost
model, assembled here from the issue record; (B) the design of the
measurement campaign that needs a prod-shaped copy — gated on the team
lead's word per docs/agents/prod-box-reads.md, queued as a bulk question.

## A. What is already measured or structurally known

**The concurrency budget.** 8 reader connections on the API pool
(crates/app/src/main.rs), 4 vCPUs, 8 GB RAM. Three isolation moats exist:
`/v1/sql` has its own pool (SQL_READERS) + per-token limiter (2 concurrent,
300/h) + 10s hard cap; walk-shaped REST filters route to the isolated
runtime (issue 120/17, landed 2026-08-08); everything else shares the 8
readers. SSE snapshots take a pooled reader per page (issue 55 fix) but
bypass the walk-routing (issue 163, open).

**Cancellation reality (issue 120, measured).** A warm walk never yields;
drop-based timeouts do not fire mid-statement; an abandoned query runs to
completion server-side. The 10s SQL cap is enforced by drop + backstop, so
a 408'd query's work continues — measured again tonight (a COUNT over
v_tenders 408'd and kept running). Consequence: every capacity number must
assume worst-case queries run to completion once admitted; admission is the
only control point.

**Point costs measured tonight (rev 0bac0e6, 8.1M tenders, 13.2M lots):**
- `GET /v1/tenders/{id}` (single-lot): 0.70–0.80s cold, ~19ms warm-path list.
- `GET /v1/tenders?limit=20` (+country filter): 0.56–0.61s.
- `GET /v1/lots?tender=`: 0.67s.
- `/health`: 0.5–0.7ms (no pool contention); it degraded to unreachable
  during both saturation events below — it shares the process, so it is a
  saturation canary, not an isolated probe.
- SSE snapshot streaming: ~1.8MB / 6s per subscription over loopback
  (include_data=true), reader taken per 500-row page; five concurrent
  subscriptions transiently drove load to ~2.2 with instant recovery on
  abort (measured post-fix); the pre-fix design produced a full outage from
  the same five (incident 2026-08-09, issue 55).
- One hostile-shaped SQL (COUNT over an 8.1M-row view): consumed one SQL-pool
  reader for its full runtime (unbounded post-408); the SQL pool's 2-per-token
  concurrency is what contained it.

**Structural ceilings (no measurement needed).**
- A single process; no second reader process is possible (turso
  single-process + ADR-0005 monolith). Vertical scaling and admission
  control are the only levers.
- 5 SSE streams/IP anonymous (mod.rs client cap), ~10 rps/IP REST posture,
  governor rate limiting — all posture numbers, none derived from capacity.
- Per-change SSE cost is O(subscriptions × changes) predicate evaluations
  plus up to two `Scope::At` point reads per change per subscription
  (sse.rs diff loop) — the point reads, not the predicates, are the scaling
  term; architecture.md's "predicates are cheap" claim covers only half the
  cost.

**Known-unbounded surfaces (the campaign's targets).** (1) An SSE
subscription with include_data over a high-change-rate window multiplies
point reads; (2) walk-shaped filters via SSE (issue 163) pin main-pool
readers; (3) concurrent hostile SQL within per-token limits but across many
tokens/IPs; (4) importer projection vs API contention during the daily run
(observed benign at current volume; unquantified at rebuild volume).

## B. Campaign design (needs: prod-shaped copy + team-lead authorization)

Rig: restore a checkpoint copy of the prod DB onto scratch hardware matching
the VPS shape (4 vCPU / 8 GB), run the release binary, drive load from a
second machine. No serving-DB involvement at any point (prod-box-reads.md).

1. **Per-endpoint worst-case map**: for each REST endpoint × filter shape
   (walkable and not) × pagination depth, measure cold/warm latency and CPU
   at concurrency 1, 4, 8, 16 — outputs a cost table keyed like the
   walk-router's shapes.
2. **SSE fan-out curve**: N subscriptions (10 → 500, mixed filters,
   include_data on/off) against a replayed change stream at the daily-run
   rate and at 10×; measure delivery lag, reader-pool occupancy, RSS.
   Success criterion: find N* where lag exceeds 5s — that is the real
   per-box subscription budget the 5/IP cap should derive from.
3. **Hostile SQL swarm**: k tokens × 2 concurrent 10s-capped worst-case
   queries (deep scans within the allow-list); measure REST p99 alongside —
   validates or resizes SQL_READERS and the per-token numbers.
4. **Mixed-load soak**: daily-run projection replay + realistic API mix for
   1h; assert /health p99 < 100ms throughout (the operational SLO tonight's
   incidents suggest).
5. **Deliverable**: capacity-budget doc: sustainable rps per endpoint class,
   SSE subscription budget, SQL concurrency budget, and the derived rate
   limits — replacing today's posture numbers with measured ones. Any gap
   between budget and launch expectations becomes sizing or admission work,
   named before launch instead of during it.

Estimated effort: setup half a day (checkpoint copy exists as a runbook
path — turso-scale.md backup section), campaign one day, write-up half a day.

## Open questions

- Whether the Hetzner account has scratch capacity for the rig, or a
  one-off rental is needed (cost question for Lennart alongside the
  authorization).
- Whether to fold issue 163's fix (SSE walk-routing) in before the campaign
  so it measures the corrected design.
