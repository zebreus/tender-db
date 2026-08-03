# 120 — the app has no defence against expensive-query saturation

Status: open — architectural gap, established 2026-08-03 while costing issue 117 Class B.
Kind: availability / architecture
Blocked by: —
Blocks: —
Priority: medium — no live incident, but it is the reason every DoS-shaped read defect so far has had to
be fixed one query at a time.

## The gap

**The public API has 8 reader connections, and nothing can stop a query once it starts.** Those two
facts together mean any single expensive query saturates the API, and the app cannot intervene.

Measured, both on the deployed turso 0.7.0:

- **`READERS = 8`** (`crates/app/src/main.rs`) — "the API's real concurrency".
- **A walk cannot be aborted.** 4M rows, filter matching nothing, a 50 ms `tokio::time::timeout`
  wrapped around the read exactly as an app-layer guard would be written:

  | runtime | walk | budget | returned after |
  |---|---|---|---|
  | current_thread | 0.4408 s | 50 ms | 0.4426 s |
  | multi_thread (production's flavour) | 0.4485 s | 50 ms | 0.4533 s |

  The timeout never fires. `Statement::step` yields only on IO, so a warm walk never returns to the
  executor, and `timeout` polls the inner future and the sleep from the *same task* — so more worker
  threads do not help. Probe: `crates/store/tests/scan_budget_probe.rs`.
- **An abandoned request keeps running** — ~65 CPU ticks per 10 s window for 60 s+ after the client was
  killed, against a 0-tick idle baseline (run-driver). A disconnect frees the socket, never the work.

## Why this is bigger than any one query

Issue 117's `?country=` matches-late is *one instance*. The class is "any query whose cost the planner
and the app both fail to predict", and every member has had to be found and fixed individually — the
2.2 s lots walk, the 248.8 s `tender_detail`, the 22 s / 99 s / 226 s / >380 s paginated filters.

The reason they had to be fixed one at a time is precisely this gap: **there is no backstop.** A system
that could bound per-request work would have degraded gracefully on all of them instead of serving
minutes-long responses, and the fixes would have been performance work rather than availability work.

An app-layer budget cannot supply that backstop:

- it cannot **abort** — measured above;
- it can only **refuse before starting**, which needs a cost oracle predicting *this query will be
  expensive*. We have none. There are no selectivity statistics, and the one probe we do have (issue
  117's existence short-circuit) answers only "does any row carry this value", which is the
  matches-*nothing* case. For matches-*late* the value exists, so nothing pre-flight distinguishes it
  from a cheap query.

## The only true fix, and its cost

**Cancellability.** `turso::Connection` exposes no `interrupt()`; the capability exists one layer down
in `turso_sdk_kit::rsapi` (established by run-driver while assessing issue 117). Reaching it means
vendoring or forking a pinned dependency.

That is a permanent obligation on the engine we deploy, taken on to fix a class rather than an
instance. It is the right shape and possibly not the right trade — which is why this is filed as its
own item rather than folded into 117, where it would have been decided as a side effect of a
ten-second defect on a rare filter.

## What is being done instead, and its limit

Issue 117's residual (~10 s on a rare-but-real code) is accepted and bounded **at the ingress** with
`limit_req`, not in the app. That control is proportionate for that defect, and its own limitation must
be recorded with it: **`limit_req` sees arrival rate and cannot see that every admitted request holds a
reader to completion.** So the limit is derived, not chosen —

```
admitted_rate x walk_duration < pool_size
```

— which at 8 readers and a ~10 s walk means well under 0.8 req/s per key. A limit picked as a
plausible-looking requests/sec would look protective and not be.

**Note the ingress control does not close this issue.** It bounds one known defect's amplification. It
does nothing for the next expensive query nobody has found yet, because rate-limiting cannot
distinguish a cheap request from an expensive one either.

## Verification note

`READERS = 8` is read from source. Before any limit is derived from it, **confirm the value in the
deployed configuration** — an environment override would silently invalidate the arithmetic. Know the
inputs of the derivation, not just its form.
