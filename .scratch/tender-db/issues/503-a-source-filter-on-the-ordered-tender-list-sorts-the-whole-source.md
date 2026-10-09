# 503 — a `source` filter on the ordered Tender list sorts the whole source

Status: ready-for-agent — filed 2026-10-09 from the hourly audit (issue 494 unit 2's read of the access log).
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
