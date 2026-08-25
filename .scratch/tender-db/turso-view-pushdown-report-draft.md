# DRAFT upstream report for turso (issue 239) — Lennart files it if he agrees

Not filed anywhere yet. Repo: tursodatabase/turso. Suggested title:

**Predicates are not pushed into views — a PK-filtered single-table view is
~1000× slower than the same filter on the table**

## Body (ready to paste)

On turso 0.7.2, filtering any view materialises the whole view first; no
predicate is pushed down, including a plain primary-key equality on a
single-table view. Ordinary SQLite flattens all of these.

Measured on a ~7.9M-row table (same DB, warm, repeated):

```sql
SELECT current_seq FROM tenders WHERE id = 93601;          -- 0.001 s (table)
SELECT seq FROM v_tender_current WHERE tender_id = 93601;  -- 1.41 s  (single-table view over tenders)
-- the view's own join written raw, same PK filter:        -- 0.017 s
SELECT id, title FROM v_tenders WHERE id = 93601;          -- >10 s   (same join, in a view)
SELECT COUNT(*) FROM v_tenders WHERE id < 5000;            -- >10 s   (0.06 % id range)
```

`v_tender_current` is `SELECT tender_id, current_seq AS seq … FROM tenders
WHERE current_seq IS NOT NULL` — nothing that should block flattening (no
aggregate, no LIMIT, no window). EXPLAIN QUERY PLAN shows the view scanned in
full and the filter applied afterwards.

Happy to provide schema + a self-contained repro script if useful.

## Version pin note for our side (already in the repo)

The behaviour is re-confirmed under 0.7.2 and pinned by a plan-level tripwire
(issue 239); the `v_*` docs say NOT FILTERABLE and point at base-table joins.
When a turso release notes view flattening/pushdown, re-run the tripwire and
re-open 50's "views as the analyst entry point" if it passes.
