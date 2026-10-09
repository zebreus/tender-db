# 503 — a `source` filter on the ordered Tender list sorts the whole source

Status: done — UNIT 3 DEPLOYED 2026-10-09 (`3c80925`, gate green; review `wf_acae9505-1b8`, its one finding fixed). `source_pin`
picks the index that bounds the read: `status=open` → the deadline index (first, whatever comes with it); a closed range
on the other date column → that column's index; a one-sided other bound or a value band → the planner; otherwise the
ordering column's own index. Re-timed on prod, warm, TED, limit 50:

| Request | Before | After |
|---|---|---|
| `sort=published_at&status=closed` | 17.2 s | **7 ms** |
| `sort=deadline&status=closed` | 11.7 s | **134 ms** |
| `sort=published_at&status=open` | 4.7 s | **78 ms** |
| `sort=deadline&status=open` | 85 ms | **6 ms** |
| `sort=published_at&deadline_after=2026-11-01&deadline_before=2026-12-01` | 4.7 s | **24 ms** |
| `sort=deadline&published_after=2026-09-01&published_before=2026-10-01` | 4.6 s | **111 ms** |
| `sort=deadline&status=open&published_after=2025-10-01&published_before=2026-10-01` | — | **6 ms** |

Residual, left on purpose: a one-sided bound on the OTHER column (`sort=deadline&published_after=2026-01-01`) stays
with the planner at about 4.9 s. An open-ended range has no known size, and pinning either index can read most of TED.
It would need a size estimate before choosing; reopen if a consumer hits it.

Units 1+2 DONE 2026-10-09: DEPLOYED (`b568c10`, gate green). The boot detector queued
Reindex 2101, which built both indexes in 41 s.
- **Re-timed on prod**, warm (cold):

  | Request | Before | After |
  |---|---|---|
  | `source=ted&sort=published_at` | 17.5 s | **8 ms** (24 ms) |
  | `source=ted&sort=deadline` | 11.4 s | **142 ms** (193 ms) |
  | `source=doe&sort=published_at` | 1.1 s | **6 ms** |
  | `source=fts&sort=deadline` | 1.0 s | **27 ms** |
  | `source=nope` | 34 ms | **1 ms** |
  | `source=ted&sort=published_at&published_before=2020-01-01` | — | 15 ms |

  The deadline ordering sits at the unfiltered deadline list's own level (`sort=deadline&order=asc`, 115 ms):
  its remaining cost is the per-row satellite reads on old Tenders, not the source.
- **Review** `wf_133db53c-649` confirmed two findings, both fixed in `b568c10`:
  - The pin is skipped when the other date column, a value band or a status could drive the read.
  - A rebuild-time "no such index" reruns the read unpinned.
- Unit 3 (now done, above) was filed for the combinations left to the planner, about 4.7 s each:
  - `source=ted&sort=deadline&published_after=2026-09-01&published_before=2026-10-01`: 4.6 s;
  - `source=ted&sort=published_at&status=open`: 4.7 s.

  Both have a bounded range on the OTHER column's new composite index: `(source, current_published_at, id)`
  for a published range, and `(source, current_deadline, id)` for `status=open`, which is `current_deadline >
  now`. Pin that index instead: a range seek, then a sort of the range. Keep `status=closed` (an OR) and value
  bands unpinned. Re-time both and add them to `ordered_source_plan.rs`.
- **Confirmed on prod first** through `/v1/sql`, on the API's exact window. Plain `t.source = 'ted'` hit the 15 s
  limit (408). `+t.source` answered in 0.30 s.
- **An index alone does not steer turso.** On a fresh DB the planner walks `tenders_current_published` for this
  window, with or without ANALYZE, and with `(source, key, id)` present. Prod's planner takes `tenders_source_id`
  and sorts. Neither picks the composite index on its own once `tender_versions` is joined. It does pick it on a
  bare `tenders` query.
- **The fix:**
  - `DEFERRED_TENDER_INDEXES` gains `tenders_source_published (source, current_published_at, id)` and
    `tenders_source_deadline (source, current_deadline, id)`.
  - `read::tenders_ordered` pins one with `INDEXED BY` when the filter names a source, the FROM is the plain
    `tenders t` (seeded reads drive from their `hits` set) and the index exists. That is one `sqlite_master`
    lookup per request: `INDEXED BY` errors on a missing index, and a deploy builds none (issue 111).
- **Test** `ordered_source_plan.rs`:
  - The window seeks the pinned index on `source=?` with no sorter, for ted, doe and nope, both directions,
    first and cursor pages, both orderings.
  - Every page through the real `tenders_ordered` equals the source's rows in `(key, id)` order: 4 sources ×
    2 orders × 2 directions, with ties and NULL deadlines.
  - A seeded read is not pinned.
  - With the index dropped the read falls back and answers identically.
Built as option 1 with an explicit pin (unit 1+2 notes):
Kind: performance / the REST read path (`crates/store/src/read.rs` `tenders_ordered_query`)
Relates to: 216 (the ordered list), 117 / 120 (isolation routing), 273 (the ids-only window), 494 (the
latency gauges; nginx's `rt=` field is what found this)

## What is wrong

`GET /v1/tenders?source=ted&sort=published_at&limit=50` is one of the two flagship orderings with the most
common companion filter. On prod, on an idle box (no job running, 2026-10-09 ~09:20 UTC):

| Request | Cold | Warm |
|---|---|---|
| `source=ted&sort=published_at&limit=50` | **503 after 25 s** | **17.5 s** |
| `source=ted&sort=deadline&limit=50` | — | **11.4 s** |
| `source=doe&sort=published_at&limit=50` | 2.1 s | 1.1 s |
| `source=fts&sort=deadline&limit=50` | — | 1.0 s |
| `source=nope&sort=published_at&limit=5` (absent) | — | 0.034 s |
| `sort=deadline&order=asc&limit=10` (no source) | 0.32 s | 0.12 s |

nginx's log has the same shape from 2026-10-08 19:16 UTC (`source=ted&sort=published_at` 17.0 s). The docs
table (`v1/docs.rs:656`) promises 2–13 ms for the ordered list.

## Why (diagnosed, plan not EXPLAINed)

`tenders_ordered_query` puts `AND t.source = ?` into the ids-only window beside `ORDER BY {key} DESC, t.id
DESC LIMIT ?`. `t.source` is an equality on `tenders_source_id (source, id)`. The planner takes that index,
reads every Tender of the source, and sorts them all by the ordering key before the LIMIT. For TED that is
most of the 4.26M Tenders.

The same window with the index use suppressed (`+t.source = ?`) rides `tenders_current_published`
newest-first. Measured through `/v1/sql` (bounded, LIMIT 50; the times are net of a 1.63 s ssh +
`SELECT 1` baseline): **ted ≈ 70 ms, doe ≈ 55 ms, fts ≈ 40 ms**, against 17.5 s.

## Why it is not a one-character fix

`+t.source` turns the absent and sparse cases into walks. `source=nope` is 34 ms today because the
`tenders_source_id` seek finds nothing. With `+`, it walks all of `tenders_current_published` on the MAIN
pool: `walks()` does not route `Isolated::Source` for Tenders, because the id-ordered shape is
index-served, and the API does not validate `source` against a known set.

## Options

1. **Composite indexes**: `tenders(source, current_published_at, id)` and `tenders(source, current_deadline,
   id)`, built through `DEFERRED_TENDER_INDEXES`. They serve the equality and the order together for every
   density, absent included. Cost: two more indexes maintained on every head update, and a boot-time build
   of about 4.3M rows each.
2. **`+t.source` on the ordered window only**, plus an absent-value guard (`reachable` probing
   `tenders_source_id` with `LIMIT 1`, so `source=nope` short-circuits). Then either route `Isolated::Source`
   for the ordered shape, or accept sparse-source walks: today every source (ted, doe, fts) publishes
   daily, so the newest-first walk fills a page quickly.
3. Validate `source` against the known set (400 on an unknown value) in either option. It also stops the
   absent-value case at the door.

Lean: option 1 (no density cliff, no new routing), unless the index build or maintenance measures badly.

## Units

1. A plan pin in the 112/114 style, against `tenders_ordered_statement` with a source companion: the window
   must not sort the source's whole slice (EXPLAIN shows no `USE TEMP B-TREE FOR ORDER BY` with
   `source`). It fails today.
2. The fix (option 1 or 2) and a re-time on prod of the six requests above. Done when `source=ted` on both
   orderings is under 100 ms warm, and `source=nope` stays fast.
