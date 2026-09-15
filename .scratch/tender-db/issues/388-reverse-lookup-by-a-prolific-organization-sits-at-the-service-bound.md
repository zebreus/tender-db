# 388 — a reverse lookup by a prolific bidder/winner org re-pays the org's whole participation set on every page: 2.3–4.7 s warm, 15–28 s cold, against a documented sub-25 ms contract

Status: needs-triage — filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
Kind: defect (read layer — the participation reverse-lookup seed; performance and availability)
Relates to: 223 (RESOLVED "every `winner=`/`bidder=` lookup is sub-25 ms" — that is the contract this
breaks, and its residual section states the premise that fails here: "an org's row count is bounded by
how often it won / bid — small"; the `/v1/lots?winner=` 503 instance from the same fan-out is being
appended to 223 separately, so this issue is the wider cost measurement, not that instance), 225
(RESOLVED — the same non-covering-seed shape, diagnosed and fixed for `buyer=` with a covering
`(organization_id, role, tender_id)` deferred index; its closing line "every participation
reverse-lookup (buyer/winner/bidder) is sub-second for every org class tested" did not test this
class), 273 (the 30 s bound → 503 class this sits 1.8 s from), 120 (the isolated pool these requests
occupy), 117 (the walk class, and the cursor-in-the-index rule the fix needs), 111 (the deferred-index
builder that would create the covering indexes without a rebuild), 112 (the standing gate that the
deferred indexes serve their reads)
Blocked by: nothing

## Observed (verified 2026-09-13 on prod)

Org `357` = `https://tenders.zebreus.click/v1/organizations/357` → "ALLIANCE HEALTHCARE ROMÂNIA SRL",
RO, **193,890 mentions** (4th-most-mentioned org on page 1 of `/v1/organizations?limit=1000`; org
`355` FARMEXIM S.A. has **171,003**).

Measured 2026-09-13 20:30–20:45 UTC against rev `9e082fd1893156446eb50dc880a5d2052f21c532`, one
request at a time, no concurrency, each line

```
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total} ttfb=%{time_starttransfer}\n' --max-time 60 '<url>'
```

`/health` measured 0.65 s, so ~0.6 s of every figure below is proxy RTT.

| # | url | status | time_total | ttfb | note |
| --- | --- | --- | --- | --- | --- |
| 1 | `https://tenders.zebreus.click/v1/tenders?bidder=357&limit=1000` | 200 | **28.190419** | 27.682605 | cold; 1000 items, `next_cursor "859213"` |
| 2 | same url, re-run immediately | 200 | 4.000407 | 3.490414 | warm |
| 3 | `https://tenders.zebreus.click/v1/tenders?bidder=357&limit=1000&cursor=859213` | 200 | 4.669266 | 4.355454 | page 2 = last page, 376 items, `more=false` |
| 4 | `https://tenders.zebreus.click/v1/tenders?bidder=357&limit=100` | 200 | 3.092392 | 2.890952 | typical page size — same cost |
| 5 | `https://tenders.zebreus.click/v1/tenders?winner=357&limit=1000` | 200 | **24.162444** | 23.653839 | cold; byte-identical 556,138-byte page to (1) |
| 6 | `https://tenders.zebreus.click/v1/lots?bidder=357&limit=1000` | 200 | 9.772883 | 9.463906 | |

Controls in the same session, same rev:

| control | time_total |
| --- | --- |
| `/v1/tenders?buyer=28&limit=1000` (org with 372,628 mentions, non-buyer role) | 0.87 s |
| `/v1/tenders?country=DE&cpv=45&limit=1000` | 1.19 s |
| `/v1/tenders/6978910` (2,604 lots, 2.1 MB) | 1.51 s |
| `/v1/tenders?min_value=1000000000000&limit=1000` (full-corpus walk, 234 rows) | 2.09 s |

### Re-run 2026-09-14, same rev (`/health` RTT 0.61 s, 04:20–04:24 UTC, sequential)

The fixed per-page cost reproduces, and it does not move with page size or cursor depth:

| request | time_total |
| --- | --- |
| `/v1/tenders?bidder=357&limit=1` | 2.32 s |
| `/v1/tenders?bidder=357&limit=100` | 2.47–2.79 s |
| `/v1/tenders?bidder=357&limit=1000` | 3.96–4.11 s |
| `/v1/tenders?bidder=357&limit=1000&cursor=859213` | 3.94 s |
| `/v1/tenders?winner=357&limit=1000` | 15.40 s cold, then 4.36 s warm (byte-identical 556,138-byte page) |
| `/v1/lots?bidder=357&limit=1000` | 9.79 s |
| `/v1/tenders?bidder=355&limit=100` | 6.32 s cold, 2.55 s warm |
| `/v1/tenders?winner=355&limit=100` | 3.96 s |
| `/v1/tenders?buyer=28&limit=1000` | 0.89 s |
| `/v1/tenders?bidder=28` | 0.43 s |
| `/v1/tenders?bidder=23494787` (the org issue 223 verified at 10 ms) | 0.42 s |

**What did NOT reproduce:** the 24–28 s cold figures did not recur in the second session — the coldest
observation there was 15.40 s — and **no request returned 503**. The cold-503 risk is a consequence of
the 28.19 s reading in the first session, not something either session observed. Direct row counts on
`tender_version_bid_parties` were refused by the permission classifier; mention counts and page sizes
stand in for them.

### One bounded box read (the judge's, on the serving row layout)

```sql
SELECT COUNT(*), COUNT(DISTINCT tender_id) FROM tender_version_result_winners WHERE organization_id = 357
```

→ **3,551,573 rows over 3,668 distinct tenders** — ~968 rows per tender (versions × lot results). That
is the number the seed materialises, in full, on every page.

### Mechanism (source)

- `participation_seed` (`crates/store/src/read.rs:1010`) seeds `tender_from` (`:1756`) and `lot_from`
  (`:2550`) with `(SELECT DISTINCT tender_id FROM <table> WHERE organization_id = ?) hits JOIN tenders t ON t.id = hits.tender_id`.
- `tenders_page_query` (`:1962`) applies `t.id > ? ORDER BY t.id LIMIT ?` to the join, **outside** the
  seed, so the whole `DISTINCT` is rebuilt per page and page size cannot reduce it. The per-row
  `EXISTS` then re-checks membership.
- The only indexes on both tables are `(organization_id)` alone (`crates/store/src/canonical.rs:712`,
  `:772`; deferred catalogue `:6897-6898`), so the seed pays one table lookup per participation row
  just to read `tender_id` — it cannot be answered index-only. Exactly the shape 225 fixed for
  `buyer=`; `winner`/`bidder` were left out on `participation_seed`'s own doc line: "their tables are
  participation-bounded already".

## Why it matters

A consumer paging a supplier's tenders gets the wrong answer about what the API costs, in three ways:

| | |
| --- | --- |
| the full answer for org 357 | 1,376 tenders over 2 pages ≈ **8 s warm, ~50 s cold** |
| tuning the page size | does nothing — `limit=1` costs 2.32 s and `limit=1000` costs 3.96 s |
| paging deeper | does nothing — the `cursor=859213` page costs 3.94 s, the same as page 1 |
| the documented cost | issue 223: "every `winner=`/`bidder=` lookup is **sub-25 ms**"; issue 225: "**sub-second** for every org class tested" |

`Winner` and `Bidder` are in the `Isolated` enum, so `walks()` routes these to the isolated pool
(`crates/app/src/v1/isolate.rs:105`, 4 slots, global and unauthenticated). Four concurrent legitimate
`bidder=357` requests hold every slot for tens of seconds and shed everything else on those endpoints —
the same cheap-brown-out class as issue 273. And `REQUEST_DEADLINE` is 30 s
(`crates/app/src/v1/mod.rs:408`): the 28.190419 s reading is **1.81 s** from the point where a valid
query stops returning results and starts returning 503.

## Why this is ours, not the publisher's

The publisher publishes a pharma supplier that bids on multi-hundred-lot, many-version drug tenders —
tender 6978910 (2,604 lots) is exactly that shape, and nothing about it is malformed. What turns it
into 3.55M winner rows over 3,668 tenders is our own row layout (one row per version per lot result),
and what makes every page re-read all 3.55M of them is our query shape and our index choice: a
role-blind `SELECT DISTINCT tender_id` materialised before `ORDER BY … LIMIT`, over an index covering
`organization_id` alone. Issues 223 and 225 wrote the opposite premise down ("bounded … small",
"tens–thousands of rows", "participation-bounded already") and never tested an org of this class; the
premise is falsified by our storage, not by anything published.

## Repro

Under two minutes, read-only, no box access needed:

```
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total}\n' --max-time 60 'https://tenders.zebreus.click/v1/tenders?bidder=357&limit=1'
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total}\n' --max-time 60 'https://tenders.zebreus.click/v1/tenders?bidder=357&limit=1000'
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total}\n' --max-time 60 'https://tenders.zebreus.click/v1/tenders?bidder=357&limit=1000&cursor=859213'
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total}\n' --max-time 60 'https://tenders.zebreus.click/v1/tenders?bidder=23494787&limit=1000'
curl -sS -o /dev/null -w 'status=%{http_code} time_total=%{time_total}\n' --max-time 60 'https://tenders.zebreus.click/health'
```

Lines 1–3 all land in the 2.3–4.7 s band (cold on a fresh cache: 15–28 s); line 4, the org issue 223
measured at 10 ms, returns in ~0.4 s; line 5 is the ~0.6 s proxy RTT floor to subtract from each. The
first request of the day against `winner=357&limit=1000` is the cold case — run it before anything
else warms the page cache if the cold figure is what needs re-measuring.

## Done when

- `tender_version_result_winners(organization_id, tender_id)` and
  `tender_version_bid_parties(organization_id, tender_id)` exist in `DEFERRED_TENDER_INDEXES`
  (`crates/store/src/canonical.rs:6897-6898`, the 225 recipe) and are built by the issue-111 builder,
  and `EXPLAIN QUERY PLAN` for both seeds shows an index-only search with no table access.
- The page cursor is applied INSIDE the seed (walk the covering index in `tender_id` order, group,
  `LIMIT`) so a page's cost scales with the page, not with the org: `?bidder=357&limit=1` is
  measurably cheaper than `?bidder=357&limit=1000`, and the `cursor=859213` page is not more expensive
  than page 1.
- Prod, network-inclusive, warm: `/v1/tenders?bidder=357`, `?winner=357`, `?bidder=355`, `?winner=355`
  and `/v1/lots?bidder=357` each return in **under 1 s** at `limit=100` — the contract 225 closed on —
  and the cold first request is under 3 s.
- The controls stay where they are: `bidder=23494787` ≤ 0.42 s, `bidder=28` ≤ 0.43 s,
  `buyer=28&limit=1000` ≤ 0.89 s, and the `winner=357&limit=1000` page is still byte-identical
  (556,138 bytes) to today's.
- A gate pins the class rather than the instance: a test that fails if either participation seed is
  planned without its covering index (issue 112's shape), so the next reverse-lookup filter cannot
  ship on an `(organization_id)`-only index.
- Issue 223's status line and `participation_seed`'s doc comment ("their tables are
  participation-bounded already") are corrected to say what is actually bounded and by what, with the
  357/355 numbers, so the premise is not re-derived from the old prose.
