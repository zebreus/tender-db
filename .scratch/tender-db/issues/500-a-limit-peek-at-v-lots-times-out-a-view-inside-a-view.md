# 500 — `SELECT * FROM v_lots LIMIT 5` times out: a view joined inside a view scans every Tender per row

Status: ready-for-agent — FIX BUILT 2026-10-08 with a plan test. NEXT: gate, deploy, and re-time the three
peeks on prod (each expected well under 1 s).
Kind: performance / the SQL surface (the main consumption path)
Relates to: 494 (its slow-query log found this the day it shipped), 239 (views are not filterable), 25
(`v_tender_current`)

## What was wrong

Issue 494's slow log on its first evening (2026-10-08, deploy B):

```
15.007s status 408 user 17: SELECT * FROM v_lots LIMIT 5
15.006s status 408 user 18: SELECT * FROM v_lots LIMIT 3
 9.286s status 200 user 18: SELECT * FROM v_lot_results LIMIT 2
 9.291s status 200 user 18: SELECT * FROM v_awards LIMIT 2
```

These are a consumer's first look at the data, and `/docs` calls the views "the readable way to see the
shape". `v_tenders`, `v_organizations` and the base-table bodies answered the same peeks in about 0.2 s.

**Cause.** turso does not flatten a view inside a view.

- `v_lots` and `v_lot_results` joined the `v_tender_current` view. Their plan scanned all of `tenders` once
  PER lot or result (`SCAN lots` → `SEARCH vl` → `SCAN c` / `SCAN tenders`).
- `v_awards` read `v_lot_results`. Its two buyer subqueries over the `v_tender_buyers` view scanned every
  `tender_version_parties` row per award.

Not new with deploy B: the joins date from issue 25.

## Fix

- `v_lots` and `v_lot_results` read the head from `tenders.current_seq` by key. The inner join on
  `seq = current_seq` drops a NULL head exactly as `v_tender_current`'s `WHERE` did.
- `v_awards`' buyer subqueries are `v_tender_buyers`' body with the Tender pushed in by key. They seek
  `tender_version_parties_version`, whose (tender_id, seq, rowid) order yields the same first buyer the view's
  rowid scan did.

**Measured on prod through the rewritten bodies** (the live views shown for comparison):

| Query | Rewritten | Live view |
|---|---|---|
| `v_lot_results` peek | about 0.3 s | 9.3 s, or 408 at LIMIT 3 |
| `v_lots` body | about 0.4 s | 408 |
| `v_awards`, LIMIT 2 | about 0.3 s | 9.3 s |

`v_awards` LIMIT 2 returned identical rows from both. `v_lot_results` could not be compared row for row,
because the old view times out; it is equivalent by construction.

**Test:** `an_unfiltered_peek_at_any_view_scans_one_base_table` (`crates/store/tests/view_pushdown_probe.rs`).
For every view, the plan of `SELECT * FROM v LIMIT 3` may scan at most one base table, its driver. It failed
on the old schema (`v_lots` scanned `lots` and `tenders`; `v_awards` scanned `tender_version_lot_results` and
`tender_version_parties` twice).

## Units

1. **Gate and deploy.** The views are recreated at boot, so no migration is needed. Re-time the three peeks
   and read the slow log for a day.
2. **Doc check.** Whether `/docs` should say outright that a `LIMIT` peek at any view is cheap while a filter
   on one is refused. Decide after unit 1's numbers.
