# 25 — /api/tenders cold read is ~3s (v_tenders MAX-seq view)

Status: RESOLVED (verified 2026-08-16, owner sweep). This issue is the origin of the current_seq/current_published_at head pointer + tenders_current_published index, which shipped long since; the newest-tenders read is an index range scan. Measured today: /v1/tenders?sort=published_at&limit=200 in 0.083 s against the 7.9M-tender corpus (and 2.4 ms for a 5-row page).
Observed post-bad8dda deploy (2026-07-21): `/api/tenders` (list_tenders,
200 rows over the `v_tenders` MAX(seq)-per-tender view) took 2.9s on a
cold read at 3.5M notices; warm reads are fast. The dashboard hides it
behind SSR + caching today, but the cost is O(all tender versions) per
cold read and the dataset is about to quadruple with the backfill.

Investigate: what plan does turso pick for the view's MAX(seq) GROUP BY;
whether a current-version flag/table (maintained by the projection, which
already knows when it supersedes a version) or an index gets it to
O(page). Prefer the design that keeps ADR-0001's append-only change
model intact.

Acceptance: cold /api/tenders and /v1/tenders p99 < 500ms at full-archive
scale.

## Investigation

`v_tenders` = `tenders JOIN v_tender_current JOIN tender_versions`, where
`v_tender_current` is `SELECT tender_id, MAX(seq) FROM tender_versions GROUP
BY tender_id`. `list_tenders` (the /api/tenders SSR path) does
`SELECT ... FROM v_tenders ORDER BY published_at DESC, id DESC LIMIT 200`.
Two costs, both O(all): the `MAX(seq)` aggregation scans every version, and
the ORDER BY is on the *current version's* `published_at` — a derived column
no index can serve — so every current-tender row is materialised and sorted
per cold read. No pure-index fix exists; the order key must be materialised.

`/v1/tenders` (read.rs `tenders`) is separate and already O(page): it keyset-
paginates by `t.id` (`t.id > ? ORDER BY t.id LIMIT`) and picks the current
seq with a per-row `MAX(seq)` correlated subquery off the PK — left as-is
(self-correcting, meets the target even at 4× scale).

## Fix (2026-07-21) — maintained current-version pointer

A denormalised head pointer on `tenders`, `current_seq` +
`current_published_at`, maintained by the projection (it already knows the
head when it writes a version — `apply_tender_tx` sets it after the write
loop). `tender_versions` stays fully append-only; ADR-0001 explicitly allows
validity-range writes on the canonical layer, and full history is retained,
so the append-only change model is intact.

- **Schema**: two nullable columns on `tenders`; index
  `tenders_current_published(current_published_at, id)`.
- **Views rewritten to read the pointer**, same public shapes:
  `v_tender_current` → `SELECT id AS tender_id, current_seq AS seq FROM
  tenders WHERE current_seq IS NOT NULL` (O(tenders), no aggregation — also
  speeds `v_lots`/`v_lot_results`/public `/v1/sql` that join it); `v_tenders`
  joins `tender_versions ON v.seq = t.current_seq`.
- **`list_tenders` rewritten** to drive straight off `tenders` ordered by
  `current_published_at DESC, id DESC LIMIT` — turso plans it
  `SCAN tenders USING COVERING INDEX tenders_current_published`, i.e. an
  ordered index scan that stops at the limit (O(page)), title resolved by one
  indexed subquery per returned row.
- **Migration/backfill**: the pointer columns are added by ALTER on a
  pre-issue-25 DB and backfilled once from `tender_versions`; the index is
  created in `migrate()` (after the column exists), not the schema batch.
  Thereafter the projection maintains them, so it is a no-op on later opens.
  Deploy note: the one-time backfill (UPDATE all tenders) + index build run at
  open on the prod DB — seconds, covered by the 120s health-check grace
  (230a952), alongside issue 20's notices(fetch_id) build.

## Tests

- `store::list_tenders_orders_from_the_index_not_a_sort` — asserts the query
  plan uses `tenders_current_published` and does no TEMP B-TREE sort (the
  O(page) guarantee, independent of dataset size).
- `store::list_tenders_orders_by_the_current_head` — newest-first ordering,
  limit as top-N, '(untitled)' fallback.
- `store::migration_backfills_the_current_version_pointer` — opening a
  pre-issue-25 schema adds + backfills the pointer (the deploy path).
- Existing ingest `project` suite (20) covers projection maintenance +
  reprocessing idempotency (unchanged chain → early return, no pointer write)
  + the `v_tenders`/`v_tender_current` assertions; app `sql`/`api` (the
  `v_tender_current` join + `/v1/tenders` version==newest) green. clippy clean.

Needs verification: cold /api/tenders p99 < 500ms in prod at full scale.
