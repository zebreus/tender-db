# 161 — Steady-state presence observation never refreshes observed_at (/health/deep falsely red)

Status: resolved
Role: app

## Symptom

`/health/deep` reports `canonical_layer: { stale: true, ok: false }` (HTTP 503)
on a perfectly healthy box, every quiet day, starting ~6h after the morning
pipeline finishes. First observed 2026-08-08 ~21:05 Berlin: `observed_at` frozen
at 07:35:41 UTC (~13.5h old against the 21600s threshold), no job running, no
observer errors or panics in the journal, service up since 08-07 with the #38
observer deployed (rev 825dd64).

## Diagnosis

The presence observer (`spawn_presence_observer`, 300s cadence) has two branches:

- heavy write in progress → `touch_layer_presence` — blanket-stamps `observed_at`.
- steady state → `observe_layer_presence` — which wrote **only when a table's
  presence state changed** ("steady state is read-only", by design).

So the freshness stamp was only ever written *during heavy writes*. In quiet
steady state (all tables `Populated`, no transitions) every 5-minute
observation was a no-op read, `observed_at` froze at the last heavy-write
touch, and the staleness clause (LAYER_STALE_SECS = 6h) flipped `/health/deep`
red ~6h after the daily pipeline — i.e. red ~18h/day since the 825dd64 deploy.
The design had it exactly backwards: it heartbeated while the writer was busy
and went silent while the writer was free.

Timing evidence: frozen `observed_at` 1786174541 (07:35:41Z) is 2s before the
morning `project` job start — the last touch tick that ran during the DÖE
`process` job (587). Every later tick took the observe branch and left no trace.

## Fix

`observe_layer_presence` now ends with the same blanket
`UPDATE layer_presence SET observed_at = ?1` that `touch_layer_presence` uses:
the observation IS the heartbeat. Verdict rows are still rewritten only on real
state transitions. Cost: one 13-row UPDATE per 5 minutes, only when no heavy
write is running.

Regression test: `a_steady_state_observation_refreshes_observed_at` (store,
canonical.rs) — observes twice with no state change and asserts the second
timestamp landed. Verified failing before the fix, passing after.

## Verification (prod)

After deploy: with no job running, `/health/deep` must show `canonical_layer.ok
= true` and `observed_at` advancing on every ~5-minute tick; overall `ok: true`
in steady state.
