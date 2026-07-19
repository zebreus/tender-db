# 07 — Read-only SQL endpoint

Status: ready-for-agent
Blocked by: 06

Goal: account holders run arbitrary read-only SQL against the canonical
schema.

Scope:
- `POST /v1/sql` (Bearer token): layered gate per docs/architecture.md —
  turso_parser single-statement SELECT allow-list (no PRAGMA/ATTACH/multi),
  dedicated `query_only=1` reader connection, 10s timeout-by-drop,
  streaming JSON rows with 10k rows / 10MB caps + truncated flag, 2
  concurrent + 300/h per token (governor + semaphore).
- Document the exposed schema (the SQL user's view: current-state views +
  version tables) in the API docs page; document the turso SQL dialect
  gaps (no recursive CTEs, partial window functions).
- Tests: allow-list adversarial suite (write attempts, PRAGMA, multi-stmt,
  CTE-wrapped writes, ATTACH), timeout behaviour, cap behaviour.

Acceptance: adversarial tests green; a real analytical query (top buyers by
awarded cents in a CPV range) works via curl with a token.
