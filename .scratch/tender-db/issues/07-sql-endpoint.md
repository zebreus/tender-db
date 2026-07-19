# 07 — Read-only SQL endpoint

Status: resolved
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

## Answer

Delivered: `POST /v1/sql` and `GET /v1/sql/schema` in a new module
`crates/app/src/v1/sql.rs`, wired into the API router with a one-line
`.merge(sql::routes())` and an `AppState.sql` field. Gated by issue 06's
`AuthUser` bearer extractor. Integration suite in `crates/app/tests/sql.rs`.

**Parser choice: `turso_parser` 0.7.0 (turso's OWN parser), pinned `=0.7.0` to
the engine.** It is already in the tree as a transitive dep of the engine, so
its AST is exactly what the engine will execute — no dialect drift, which is the
whole risk with a generic parser. api-layer.md §7 had suggested `sqlparser`
0.62 as a fallback; turso_parser proved directly usable (its `Parser` is an
iterator of `Cmd`; a single `Cmd::Stmt(Stmt::Select(_))` is the allow-list),
so the fallback was not needed. Crucially, CTE-wrapped writes parse as
`Stmt::Insert/Update/Delete`, never `Stmt::Select`, so the one type check
rejects them for free.

**The layered gate** (docs/architecture.md §API):
1. Parse → accept exactly one statement, only a bare `SELECT`. Rejects writes,
   `PRAGMA`, `ATTACH`, `VACUUM`, `EXPLAIN`, multi-statement bodies, CTE writes.
2. **Account-table deny** — an addition not in the original spec, and necessary:
   the accounts tables live in the same file, so a bare `SELECT * FROM users`
   would otherwise expose password/token/session hashes. I reject any query
   whose *identifier* tokens (from the lexer, so string literals like
   `'%sessions%'` don't false-trip) include `users`/`api_tokens`/`sessions`.
   Complete by construction: a table can only be read by naming it. Everything
   else is public business data (CONTEXT.md), so there is no positive
   table-allow-list to maintain.
3. Dedicated `query_only=1` reader pool (`SQL_READERS=4`), separate from the
   REST readers so a heavy query never starves the live API.
4. 10 s timeout-by-drop; 10 000 rows / 10 MB caps with an in-band
   `"truncated": true`; 64 KB body cap; SQL in the POST body, never the URL.
5. Per-token limits: 2 concurrent (a per-user `Semaphore`, `try_acquire` so the
   3rd is refused not queued) + 300/h (a `governor` keyed GCRA limiter). 429
   carries `Retry-After`.

**`GET /v1/sql/schema`** enumerates every table/view except the credential
tables, each with its columns (from `PRAGMA table_info`), plus notes on the
turso dialect gaps (no `WITH RECURSIVE`; partial window functions).

**A turso limitation I found and had to engineer around (worth flagging).**
`tokio::time::timeout` only fires when the wrapped future returns `Pending`, but
turso resolves CPU-bound and cached work *synchronously* (`Ready`) and only
pends on real file IO. A probe confirmed that a `generate_series` query — even a
streaming one — never yields, so the timeout never fired and the query hung.
Fix: a cooperative `tokio::task::yield_now().await` at each row boundary in the
result loop. This lets the 10 s timeout fire between streamed rows **and** stops
a long query from monopolising a worker thread — verified: a query that hung
past 120 s now stops at ~10 s. Residual gap, documented in the module: a single
non-streaming aggregate (`SELECT count(*) FROM generate_series(1, huge)`) has no
row boundary and turso exposes no `interrupt()`, so it is bounded only by the
pool size + per-token limits, not the timeout. Isolating SQL execution on its
own runtime is the eventual clean fix.

**Adversarial coverage** (`tests/sql.rs`, 8 integration + 6 unit, all green):
- `the_gate_refuses_everything_that_is_not_a_read`: INSERT, UPDATE, DELETE,
  DROP, CREATE, `PRAGMA query_only=0` (alone and smuggled after a write in a
  multi-statement body), ATTACH, VACUUM, `VACUUM INTO`, a CTE-wrapped INSERT,
  and `SELECT 1; DROP TABLE` — each → 400, and the DB is asserted unchanged
  afterwards (four chain notices, one fetch, account intact).
- `credential_tables_are_invisible`: reads of users/api_tokens/sessions (incl.
  in a subquery and double-quoted) → 400; a string literal containing the word
  → 200.
- `the_endpoint_is_account_gated`: no token / bad token → 401.
- `results_are_capped_and_flagged`: 20 000 rows → 10 000 + `truncated:true`.
- `a_long_query_is_dropped_at_the_time_limit`: a >10 s streaming query → 408.
- `a_third_concurrent_query_is_rejected`: 3 at once → exactly one 429.
- `a_real_analytical_query_answers`: the acceptance query (top buyers by awarded
  cents in a CPV range, joined across the canonical layer) → 200 with the right
  columns, plus a grouped notice count proving real data flows.
- `the_schema_endpoint_documents_the_public_surface`: v_tenders/notices present,
  credential tables absent, dialect notes present.

**Files touched:** new `crates/app/src/v1/sql.rs`, new
`crates/app/tests/sql.rs`, `crates/app/src/v1/mod.rs` (mount + `AppState.sql`),
`Cargo.toml` + `crates/app/Cargo.toml` (`turso_parser`, `governor` deps — these
landed in a concurrent commit). Nothing in `crates/ingest`.
