# 17 — Isolate SQL-endpoint execution on its own runtime

Status: ready-for-agent
Blocked by: 07

Goal: close the residual resource gap in the SQL endpoint: a single
non-streaming aggregate (no row boundary to yield on, no turso interrupt())
can hog a worker thread bounded only by pool + per-token caps.

Scope: run /v1/sql query execution on a dedicated small tokio runtime (1-2
threads) or spawn_blocking pool so a pathological query can never starve the
API/SSE/dashboard runtime; keep the cooperative per-row yield; watch
upstream for `interrupt()` exposure on turso::Connection (the engine has it
in sdk-kit; docs/research/turso-scale.md tracks it) and adopt it when
available.

Acceptance: a deliberately CPU-pathological aggregate leaves API latency
unaffected (measured in a test or documented manual probe); SQL tests stay
green.
