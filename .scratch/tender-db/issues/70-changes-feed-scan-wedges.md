# 70 — changes-feed turso-scan wedges (oldest_cursor, entity-filtered changes, filtered lists)

Status: F1+F2 fixed (this branch), F3 open
Kind: performance / availability (issue-61 class)
Blocked by: —
Relates to: 61 (the wedge pattern), the max_cursor O(1) fix (e444f8c), coverage isolation (5c48c5c)
Owner: proj-fix (diagnosis + F1/F2)

Audit for other instances of the turso-full-scan-on-the-HTTP-runtime wedge pattern
(a query turso 0.7 does not optimize, over a projection-scale table, on a runtime
thread → blocking preads starve the acceptor). Three found.

## Finding 1 — HIGH — `oldest_cursor` (FIXED)

`read::oldest_cursor` was `SELECT COALESCE(MIN(cursor),0) FROM changes` — turso
full-scans the 80M-row / ~8GB changes table for MIN. Runs on the SSE RESUME path
(sse.rs:160) on every reconnect-with-Last-Event-ID, which clients do on every blip
/ deploy → a cold 8GB scan on the HTTP runtime → wedge.

**turso surprise (verified empirically, DON'T trust EXPLAIN):** the obvious fix
`SELECT cursor FROM changes ORDER BY cursor LIMIT 1` is NOT O(1) on turso 0.7 —
EXPLAIN QUERY PLAN shows "SCAN changes" for BOTH forms, and a timing probe over 60k
rows measured IDENTICAL time (ratio 1.0): turso does not push the LIMIT into the
scan nor use the PK's natural order for early termination. So neither MIN nor
ORDER-BY-LIMIT-1 is O(1).

**Fix (shipped):** derive from the log invariant. `changes` is append-only and
NEVER trimmed (ADR-0001; no `DELETE FROM changes` exists), `cursor` is AUTOINCREMENT
from 1, so the oldest surviving cursor is **1 whenever the log is non-empty, else 0**
— and non-empty is the O(1) sqlite_sequence high-water (`max_cursor`, already proven
O(1)). `oldest_cursor = i64::from(max_cursor(conn) > 0)`.
- ⚠️ **Coupling to document/enforce:** this returns 1 for ANY non-empty log. If a
  future cursor-expiry / changes-trim path is added (CONTEXT.md: "cursors may
  expire"), it MUST maintain a real oldest watermark here (a durable value the
  trimmer updates), or oldest_cursor will under-report the trimmed floor and SSE
  resume-below-floor will stop sending the correct reset.

## Finding 2 — MEDIUM — entity-filtered `changes_since` (FIXED)

`read::changes_since` used `WHERE cursor > ? AND (? IS NULL OR entity_kind = ?)
ORDER BY cursor LIMIT ?`. The `(? IS NULL OR …)` disjunction defeats any entity
index, and there was none on (entity_kind, cursor) anyway — so a rare or NONEXISTENT
`entity_kind` with `since=0` walked the whole 80M-row changes table to collect
`limit` matches. Hit via `GET /v1/changes?entity=<rare>&since=0` (public, poll
mod.rs:581) and the SSE diff (sse.rs:246; SSE is safe — its kind comes from the
Collection enum). A public wedge.

**Fix (shipped):**
- Split `changes_since` into two planner-clean shapes: `cursor > ? ORDER BY cursor`
  (no filter, cursor PK) and `entity_kind = ? AND cursor > ? ORDER BY cursor` (uses
  the new index).
- New index `changes_entity_cursor(entity_kind, cursor)`, built **LAZILY** at the end
  of a projection (`ensure_changes_entity_cursor_index`, beside
  `ensure_unprojected_index`), NOT in the schema batch — a `CREATE INDEX` over 80M
  rows at `Db::open` would re-introduce the multi-minute slow boot issue 61 just
  removed. On prod it first builds at the next daily incremental (off the boot path).
- Guard: an `entity_kind` the projection never emits (`ENTITY_KINDS` = {tender, lot,
  organization, lot_result, bid, contract}) short-circuits to empty WITHOUT a query —
  closes the acute nonexistent-kind wedge even before the lazy index exists.
  Behaviour-identical (a nonexistent kind always yielded empty, just after a scan).
- Window before the index builds: a VALID-but-sparse kind (e.g. `contract` early)
  can still walk until the next projection builds the index. Low residual (valid
  kinds densify; the index lands within a day).

## Finding 3 — LOW/latent — filtered list endpoints (OPEN, tracked)

`read::tenders/lots/organizations/notices`: the keyset `LIMIT` bounds RETURNED rows,
not the SCAN. A selective or empty filter (country/CPV LIKE-prefix EXISTS, buyer/
winner joins, the org per-row COUNT over ~24M mentions) walks the base table before
`LIMIT` is satisfied → a slow (not necessarily wedging, but heavy) read on the HTTP
runtime. Lower severity: bounded by the result being found, and these are indexed on
their primary keyset. Fix: supporting indexes per filter column + an optional scan
cap. An indexing task — do when convenient; measure which filters actually scan at
prod scale first.

## Cleared (audited safe — do not touch)

/v1/sql (isolated runtime), webhooks (bounded batch), /health (in-memory cursor),
/v1 root + SSE snapshot (max_cursor now O(1)), coverage (isolated + change-gated).
