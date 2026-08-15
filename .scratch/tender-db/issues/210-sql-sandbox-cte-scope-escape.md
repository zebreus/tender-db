# 210 — /v1/sql sandbox escape: a same-named CTE in an inner scope whitelists a base-table read of a credential table

Status: RESOLVED — FIXED & VERIFIED IN PROD 2026-08-15. Fix in `8938e02` ("sql: scope-track CTE names
so an inner shadow can't launder a private read"), deployed (serving rev `8938e02`, health 200). The
classifier now resolves each table reference against the CTE names visible at its own lexical position (a
cons-list `Scope` threaded through the walk; each CTE body sees earlier siblings + itself when RECURSIVE,
the primary sees all siblings) instead of one global CTE set. Regression test
`cte_scope_does_not_launder_a_private_table_read` covers the confirmed exploit + later-sibling +
subquery-shadow variants and the legitimate CTE shapes; full `v1::sql` suite green (13 passed).

**Prod re-probe after deploy (bounded, count-only):**
- `SELECT COUNT(id) FROM job_queue WHERE 1 = (WITH job_queue AS (SELECT 1) SELECT 1)` → **400** "not in
  the queryable public surface" (was **200** pre-fix).
- `SELECT COUNT(*) FROM api_tokens WHERE 1 = (WITH api_tokens AS (SELECT 1) SELECT 1)` → **400**.
- `WITH t AS (SELECT 1 FROM users), users AS (SELECT 1) SELECT * FROM t` (later-sibling shadow) → **400**.
- Legit same-scope `WITH x AS (SELECT id FROM v_tenders LIMIT 1) SELECT id FROM x` → **200** with data.
- Legit nested/enclosing CTE → reached execution (408 on the heavy `v_lots` view, i.e. accepted by the
  classifier, not denied) — confirms legitimate CTEs still pass.

Was: needs-triage — **SECURITY / HIGH**, CONFIRMED EXPLOITABLE IN PROD 2026-08-15 (bounded count-only probe).
Filed from the API review (subagent, 2026-08-15).
Kind: security / correctness (the /v1/sql allow-list)
Blocked by: —
Relates to: 43 (sql-private-table-exposure — the exact class this defeats), 45 (sql-allowlist), 50 (sql-analyst-surface),
204 (pentest-the-sql-endpoint), 17 (sql-runtime-isolation)

## Symptom

An authenticated `/v1/sql` caller can read any table deliberately excluded from `ALLOWED` — `api_tokens`
(SHA-256 token hashes), `webhook_endpoints` (per-user HMAC signing secrets + private URLs), `users`,
`sessions`, `job_queue`/`job_log` — by shadowing the target's name with a CTE defined in an inner or
sibling scope. This is cross-account privilege escalation: any token holder reads every other account's
secrets. It is exactly the disclosure issues 43/204 built the allow-list to prevent.

## Confirmed (prod, count-only, no secret columns read)

```
CONTROL   SELECT COUNT(*) FROM job_queue
          → 400 {"error":{"message":"the job_queue table is not in the queryable public surface"}}

EXPLOIT   SELECT COUNT(id) FROM job_queue WHERE 1 = (WITH job_queue AS (SELECT 1) SELECT 1)
          → 200 {"columns":["n"],"rows":[[0]]}
```

`job_queue` is not in `ALLOWED`, so the plain read is denied (control). Shadowed by an inner-scope CTE of
the same name, the classifier accepts it (200). The result is computed over `COUNT(id)`, and `id` is a
base-table-only column — the shadow CTE `SELECT 1` has no `id` — so turso resolved the outer
`FROM job_queue` to the **real base table**, not the CTE. (A non-credential table was used for the probe
deliberately; the same shape reaches `api_tokens`, `webhook_endpoints`, etc.)

## Root cause — the allow-list walk collects CTE names into one flat, un-scoped set

`crates/app/src/v1/sql.rs`:

- `:607-611` — `Tables.ctes` is a single `HashSet<String>`, documented as "CTE names defined **anywhere**
  in the query." There is no notion of scope.
- `:636-642` — `walk_select` inserts every CTE's name into that one set and recurses through **all**
  subqueries, so a CTE buried in a scalar subquery contributes its name to the global set.
- `:621-630` — `disallowed_table` accepts a base-table ref iff its name is in `ctes` **or** in `ALLOWED`
  (`:627`: `!tables.ctes.contains(*name) && !ALLOWED.contains(...)`). It never asks whether a CTE of that
  name is *visible at the ref's position*.

SQL CTE visibility is lexically scoped: a `WITH` binds only its own SELECT and that SELECT's descendants.
An outer `FROM x` is an *ancestor* of an inner `WITH x AS (…)` and cannot see it, so turso resolves the
outer `x` to the base table — while the classifier, seeing the name `x` anywhere in `ctes`, waves it
through. The code comment at `:602-605` even asserts the intended invariant ("a reference resolving to a
CTE can never launder a private-table read"); the code checks name-equality, not scope resolution, so the
invariant does not hold.

The rest of the walk (`walk_expr`/`walk_table`/`walk_one_select`, exhaustive, deny-by-default TVFs) is
sound — this scope gap is the one hole.

## Fix

**Track lexical scope.** Thread a scope stack (a `Vec<HashSet<String>>`, or pass the set of CTE names
visible at each node) through `walk_select`/`walk_one_select`/`walk_table`: push a frame with the current
SELECT's CTE names on entry, pop on exit, and resolve each base-table ref against the **union of enclosing
frames** rather than one global set. A ref is allowed iff it is in `ALLOWED` or matches a CTE visible from
its own position.

No safe name-only shortcut exists: the legitimate `WITH t AS (…) SELECT * FROM t` has the CTE and the ref
in the *same* scope, while the attack has them in *different* scopes — only scope tracking distinguishes
them, so any interim guard that ignores scope would break legitimate CTE queries.

## Verification (add as a regression test in sql.rs, alongside the existing classify tests)

- The exact exploit shape must be REJECTED: `SELECT … FROM api_tokens WHERE 1 = (WITH api_tokens AS
  (SELECT 1) SELECT 1)` → `disallowed_table` returns `Some("api_tokens")`.
- Sibling-scope variant rejected: `SELECT * FROM (SELECT 1), api_tokens WHERE … (WITH api_tokens …)`.
- Legitimate same-scope CTE still ACCEPTED: `WITH t AS (SELECT * FROM tenders LIMIT 1) SELECT * FROM t`.
- Nested legitimate CTE still accepted: `WITH a AS (SELECT * FROM lots) SELECT * FROM a WHERE id IN
  (WITH b AS (SELECT id FROM a) SELECT id FROM b)`.
- Live: the count-only probe above must return 400 after the fix.

## Note

The `/v1/sql` endpoint is token-gated, so this is not unauthenticated — the blast radius is "any account
holding an API token can read every other account's credentials + all token hashes." Still HIGH: it is a
cross-tenant secret-disclosure primitive on the public API, and the mitigation the design relies on
(`ALLOWED`) is silently bypassable.
