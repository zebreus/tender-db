# 115 — tender_detail's per-lot correlated subqueries blow up on many-lot Tenders

Status: open — pre-existing perf defect, surfaced 2026-08-03 during the lots-scan latency fix (issue
diagnosed while deploying `1830d50`). NOT caused by that fix; independent and older.
Kind: performance
Blocked by: —
Blocks: the bounded-seek/truncation fix (issue 116) — must land first, or returning all lots makes this worse.

## Observation (measured, live prod, 2026-08-03)

After the lots-scan fix (`1830d50`), normal tender detail pages are ~3–5ms. But the ~16 Tenders with
very large lot counts stay minutes-slow:

- `GET /v1/tenders/7161565` (2,604 lots, capped to 1,000 served): **248.8s** measured on prod
  (`http://127.0.0.1:8080`, single hit). run-driver measured 214.6s and, pre-fix on 33dfba7, 80.8s.
  All three are single cold measurements on a contended box, so treat them as "minutes, noisy", not a
  precise before/after — but the regime is unambiguous: these Tenders take **tens to hundreds of
  seconds**, before and after the scan fix.

The scan fix removed the ~2.2s outer full-table walk; it did nothing to the dominant cost here.

## Mechanism

`read::tender_detail`'s `lots_of` query (read.rs ~857–947) carries **6 correlated scalar subqueries
per lot row** (the `pick()` subqueries over `tender_version_{texts,amounts,dates}` etc.).

**Corrected 2026-08-03 (measured — the first reading of this was too kind).** The cost is not
O(lots × 6) index lookups. The satellite tables carry exactly one index each:

```sql
CREATE INDEX tender_version_texts_version   ON tender_version_texts(tender_id, seq);
CREATE INDEX tender_version_amounts_version ON tender_version_amounts(tender_id, seq);
CREATE INDEX tender_version_dates_version   ON tender_version_dates(tender_id, seq);
```

**`lot_id` appears in no index.** So each subquery seeks the `(tender_id, seq)` slice and then walks
that slice in full, filtering `s.lot_id = l.id` row by row. The slice's size grows with the Tender's lot
count, and there is one walk per lot per subquery — so the cost is **O(lots × slice) = O(lots²)**, not
O(lots) with a large constant. Same shape as issue 92: quadratic in a per-entity quantity, invisible
until that quantity is large.

Measured locally (`crates/store/tests/lot_summary_cost.rs`, warm, release, in-page-cache — no disk in
the picture, so this is pure inner-loop work):

| lots  | whole-Tender read |
|-------|-------------------|
| 600   | 1.91s             |
| 2,400 | 30.62s            |

**16.1× the time for 4× the lots.** Linear would be ~4×; 16 is the quadratic signature. This reproduces
the entire defect on a 3,000-lot scratch database — the cost depends only on the version's satellite
slice, not on the 13.2M-row corpus — so it iterates locally without the box.

The 4.26M normal Tenders carry 1–5 lots, where lots² is nothing. That is why this hid.

Distribution (from the immutable snapshot): max lots/tender = 2,604; 16 Tenders > 1,000 lots; 424 > 500.
So the blow-up is confined to a tiny tail (16 of 4.26M), but for those Tenders the detail page is
effectively unusable and each request pins a worker for minutes (a mild resource-exhaustion surface).

## Why it matters now (the entanglement)

~~The truncation fix (issue 116 — `lots_of` reports `lots: 2604` but ships `lot_details: 1000`) wants to
return **all** the Tender's lots. But returning 2,604 instead of 1,000 multiplies THIS cost ~2.6× — the
1,000-row cap is currently the only thing bounding it.~~ **BOTH CLAIMS ARE FALSE — measured, see
"the cap bounds nothing" below.** The truncation fix is still sequenced after this one, but not for
the reason given here. So the honest-count fix is **blocked on this
one**: we cannot serve all lots until the per-lot cost is cheap. This is also why the bounded-seek shape
(`549f8f5`) was correctly NOT shipped — it removes the protective cap.

## Direction (designed 2026-08-03)

Note first that **`tender_detail` already does the right thing everywhere else**: it issues one flat
`WHERE tender_id = ? AND seq = ?` query per satellite table (read.rs 589–642) and ships those rows as
the response's `texts`/`amounts`/`dates`. `lots_of` then re-derives title/value/currency/deadline from
*the same rows* with 6 correlated subqueries per lot. The set-based shape is not a new idea in this
file — `lots()` is simply the one read that never got it.

So: split `read::lots` into a **skeleton query** and an **in-memory decoration**.

1. Skeleton — the existing `lots ⋈ tenders ⋈ tender_versions ⋈ tender_version_lots` query with every
   filter and the scope, **minus the 6 subqueries**. Yields `(id, tender_id, lot_key, kind, seq)`.
2. Decorate — for each distinct `(tender_id, seq)` present, three queries (texts with
   `field = 'title'`, amounts, dates with `field = 'submission_deadline'`, all `lot_id IS NOT NULL`),
   assembled per lot in Rust.

Tender-scoped: **3 queries, independent of lot count.** Global `/v1/lots` page: 3 per distinct version
against today's 6 per lot — strictly fewer, so no regression on the list endpoint. One code path; no
"is this a big Tender" branch; `Scope::At` rides the same route.

Semantics to preserve **exactly** (this is where a rewrite would silently drift):

- `title`: `ORDER BY (lang='ENG') DESC LIMIT 1` = the first ENG row in scan order, else the first row.
  In memory, keep-first-best: replace the incumbent only when it is not ENG and the candidate is.
- `value_cents` (`MAX(cents)`) and `currency` (`ORDER BY cents DESC LIMIT 1`) resolve to **the same
  row**, so one max-cents row serves both; ties keep the first, matching `LIMIT 1`.
- the three deadline picks share `ORDER BY utc_seconds DESC LIMIT 1`, so one max-utc row serves all
  three — if anything more self-consistent than three independent picks.

No schema change, no new index, no planner dependence for correctness.

**Rejected: add a `(tender_id, seq, lot_id)` index.** It buys a new index over three satellite tables at
prod scale plus a deferred-builder obligation (issue 111), to make a bad query shape survivable — and
the read would still be O(lots × 6) round trips. **Rejected: a `GROUP BY` join in SQL.** turso would
likely materialise it over the whole table, and that is trusting a plan again (issue 112).

Then 116 (return all lots) becomes safe, and the 16 Tenders drop from minutes to ms.

**Do NOT** "fix" this by lowering the cap or by shipping the bounded-seek — the first hides the honesty
bug, the second worsens the cost. Fix the query, then unblock 116.

## Verification note (2026-08-03 discipline)

The prod seconds quoted above are single cold measurements on a contended box — noisy, and not the
load-bearing claim. The load-bearing claim is the *shape*, and it is now measured rather than argued:
16.1× for 4× the lots, warm, in-cache, on a local scratch database.

The regression test is that measurement, as a **scaling ratio** rather than a wall-clock threshold
(`crates/store/tests/lot_summary_cost.rs`: 4× the lots must cost < 8×). A ratio is machine-independent,
so it will not rot on a faster box or flap on a contended one. It **has been run against the unfixed
code and fails** (16.1×) — it is a falsifier, not a rubber stamp. It also asserts every returned lot
carries a title, a value and a deadline, so a "fix" that quietly stops decorating cannot pass it.

Time the fix; do not read its plan (turso's EQP text misreports the `lots` access path — issue 112).


## The plan gate PASSES this read — and that is the record of why EQP cannot see 115

From issue 112's canonical run on the box (run-driver, turso 0.7.0, prod catalogue),
the verbatim plan for the `lots_of` statement the gate certified GREEN:

```
1  | 0 | 0 | SEARCH l USING INDEX sqlite_autoindex_lots_1 (tender_id=?)   <- 1830d50, working
2  | 0 | 0 | SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
3  | 0 | 0 | SEARCH vl USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=?)
4  | 0 | 0 | SEARCH v USING INDEX sqlite_autoindex_tender_versions_2 (tender_id=?)
42 | 0 | 0 | CORRELATED SCALAR SUBQUERY 1
76 | 0 | 0 | CORRELATED SCALAR SUBQUERY 2      + USE SORTER FOR ORDER BY
113| 0 | 0 | CORRELATED SCALAR SUBQUERY 3
139| 0 | 0 | CORRELATED SCALAR SUBQUERY 4      + USE SORTER FOR ORDER BY
171| 0 | 0 | CORRELATED SCALAR SUBQUERY 5      + USE SORTER FOR ORDER BY
204| 0 | 0 | CORRELATED SCALAR SUBQUERY 6      + USE SORTER FOR ORDER BY
238| 0 | 0 | CORRELATED SCALAR SUBQUERY 7      + USE SORTER FOR ORDER BY
291| 0 | 0 | USE SORTER FOR ORDER BY                                     <- see below
```

**Seven** correlated scalar subqueries — not six — five of them sorting, all
re-evaluated per lot. **Every line is green**, because every subquery is index-served
(`tender_version_texts_version`, `tender_version_amounts_version`,
`tender_version_dates_version`). There is nothing here for a plan gate to object to,
and issue 112's gate duly reports `PASS B1 lots served by index sqlite_autoindex_lots_1`.

So: **do not verify the fix to this issue with a query plan.** A plan assertion is the
wrong instrument — every plan line is already optimal and the read still takes 248.8s.
The fix needs a **clock or an execution count**. This is the EQP asymmetry from 112
stated for this issue: EQP is a sound regression detector for an access path, and no
evidence at all about how many times a good access path is taken.

It also means the read is shared: the per-lot shape is in `read::lots`, so
**`/v1/lots?tender=` carries it too**, not only `/v1/tenders/{id}`. Time both.

## The 1,000-row cap bounds NOTHING (run-driver, measured on the box)

On a faithful 2,604-lot fixture, sweeping the page size:

| LIMIT | rows returned | time |
|---|---|---|
| 125 | 125 | 16.25s |
| 250 | 250 | 16.19s |
| 500 | 500 | 16.37s |
| 1000 | 1000 | 16.05s |

**Flat across an 8× change in page size.** The cause is the last line of the plan above:
the seek yields `(tender_id, lot_key)` order while the query asks `ORDER BY l.id`, so the
top-level **sorter consumes the entire result set before `LIMIT` is applied**. All 2,604
lots × 7 subqueries run on every request no matter what page size is asked for.

Quadratic confirmed independently on two machines: 651 / 1,302 / 2,604 lots →
0.996s / 3.950s / 16.05s (4× lots ≈ 16× time).

Two consequences for this issue's plan of record:

1. The strikethrough above — the cap is not a protective bound, and lifting it to 2,604
   does **not** cost "~2.6× more". The work is already being done for every request.
2. The sorter is itself a candidate fix, and possibly the cheaper one: an ordering the
   index can satisfy would let `LIMIT` truncate before the subqueries run, which turns
   a full-set cost into a page-sized one *without* touching the per-lot shape. Worth
   measuring before committing to the batching rewrite — the two are independent, and
   the ordering change may be a fraction of the work.
