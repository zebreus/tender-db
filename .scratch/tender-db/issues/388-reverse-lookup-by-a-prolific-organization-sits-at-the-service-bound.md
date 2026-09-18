# 388 — a reverse lookup by a prolific bidder/winner org re-pays the org's whole participation set on every page: 2.3–4.7 s warm, 15–28 s cold, against a documented sub-25 ms contract

Status: **DONE 2026-09-18** — the cursor is inside the seed on both streams, DEPLOYED at `62b3ce5` 13:13 UTC and read live (closing matrix at the foot): `/v1/lots?bidder=357&limit=100` **0.71 s** (503 at 30 s when filed), `?winner=357` 0.55 s, `/v1/tenders?bidder=357` **0.90 s** (3.15 s this morning), `?winner=357` 0.74 s, a page of 1 / 100 / 1000 rows 0.42 / 0.71 / 0.92 s on lots — a page costs a page, not the org. Two clauses read honestly rather than met to the digit: `?bidder=355` 1.10 s against the 1 s line, and a deep tenders page 1.39 s warm against page 1's 0.88 s, both the satellites of the particular tenders (an unseeded page in that id region reads 0.63–0.96 s), not the walk. Was: ready-for-agent — unit 1 (the two covering indexes) LANDED 2026-09-16 and built on prod 2026-09-18 (job 1478); the seeded lots walk, the tenders half, and two same-day corrections (the head check and the window unit) landed 2026-09-18, see the comments. Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
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


## Unit 1 landed 2026-09-16 — the seeds are covered

`tender_version_result_winners(organization_id, tender_id)` and
`tender_version_bid_parties(organization_id, tender_id)` join `DEFERRED_TENDER_INDEXES`, the shape
issue 225 gave `buyer=`.

**New names, not widened definitions, and that is the load-bearing part.**
`missing_tender_indexes` filters by NAME. Re-defining `tender_version_result_winners_org` in place
would have left prod on the narrow index forever while the code and this issue both claimed the fix
had shipped — the builder would have seen the name present and skipped it. So the wide ones are
`…_org_tender` and the narrow pair stays.

### What the test could and could not prove, measured rather than assumed

The first draft asserted `USING COVERING INDEX …` and FAILED against a plan that was correct:

    SEARCH tender_version_result_winners USING INDEX tender_version_result_winners_org_tender (organization_id=?)

turso does not print the word COVERING for a SEARCH. Rather than weaken the assertion to "an index
was used" — which would pass on the narrow index too, i.e. on the bug — the test now uses the
planner's own discrimination as the evidence:

| query | index chosen |
| --- | --- |
| `SELECT DISTINCT tender_id … WHERE organization_id = ?` (the seed) | `…_org_tender` |
| `SELECT DISTINCT seq … WHERE organization_id = ?` (needs a column outside it) | `…_org` |

The wide index is chosen exactly when the query can be served from it. That also settles, with data
rather than caution, the open question of whether to DROP the narrow pair: no — the planner still
picks them for the shapes the wide ones cannot serve.

The test runs through `strip_tender_indexes` → `reset_tender_layer` → `build_tender_indexes`, because
these are deferred indexes and that is the path that creates them on prod; an index asserted only
against a fresh schema would pass while being absent where it matters.

### Still open

- **The cursor inside the seed.** The `Done when` above asks a page's cost to scale with the page
  rather than the org, which this does not do on its own: the seed still materialises the org's whole
  DISTINCT set before `ORDER BY … LIMIT`. Cheaper per row now, still O(org) per page.
- **The prod re-measurement.** The numbers in this issue are from 2026-09-13; the acceptance is
  `?bidder=357`, `?winner=357`, `?bidder=355`, `?winner=355`, `/v1/lots?bidder=357` each under 1 s
  warm at `limit=100`, with the controls unmoved. That needs the indexes BUILT on prod — they are
  deferred, so the next rebuild or the issue-111 builder creates them; until then nothing changes
  live, and this unit must not be read as the issue being fixed.

## Comment — 2026-09-18: the prod re-measurement, on the lots stream, after issue 223's IN rewrite

Issue 223's lots half deployed today (`9d41dc8`): the org seed is now `l.tender_id IN (…)`
instead of a FROM-clause JOIN. It did not move the prolific case on `/v1/lots`:

| request | seeded tenders | time |
| --- | ---: | --- |
| `/v1/lots?winner=357&limit=5` | 3,734 | `200` 22.5 s |
| `/v1/lots?bidder=357&limit=5` | 1,442 | `503` 30.6 s |
| `/v1/lots?winner=388&limit=5` | 1 | `200` 0.5 s |

Org 357 (ALLIANCE HEALTHCARE ROMÂNIA SRL, 194,301 mentions) seeds a few THOUSAND tenders, not
hundreds of thousands, and still costs 22–30 s — so the cost is not the seed's size, it is the
planner serving `ORDER BY l.id LIMIT` by walking `lots` in rowid order and probing the seed per
row until the fifth match, which for a recent, sparse org is most of the table. That is the
"cursor inside the seed" unit stated above, measured: a page's cost scales with how deep into
`lots` the org's lots sit, not with the page. `/v1/sql` refuses `EXPLAIN` — probe the plan locally
(`the_pre_115_lots_shape_still_plans_as_the_walk_we_left` is the pattern) for the IN form, the
JOIN form, and a materialised-seed form (`SELECT … FROM (SELECT … FROM lots l WHERE l.tender_id
IN (seed) AND <predicates>) ORDER BY id LIMIT ?`), and take whichever drives from the seed.

## Comment — 2026-09-18: the cost structure, measured; two fixes built; one thing this issue said had landed had not

**The plan was never the problem.** Probed locally (turso has no statistics, so the plan is
structural; `/v1/sql` refuses `EXPLAIN`): the deployed IN form drives from the org's seed —
`LIST SUBQUERY` off the `(organization_id)` index, then `SEARCH l USING INDEX
sqlite_autoindex_lots_1 (tender_id=?)`, then `USE SORTER FOR ORDER BY` — for winner, bidder and
buyer alike. The JOIN form, a materialised subquery, a `MATERIALIZED` CTE and an id-set all plan
the same way. So the 22 s was not a lots walk (the 223 comment's reading; corrected there), it was
VOLUME plus cache: `?winner=357` is 22.5 s cold and 6.1 s warm, and org 357's 3,734 winner tenders
carry **334,537 lots**, its 1,442 bidder tenders **137,564** — every one enumerated,
predicate-checked and sorted before `LIMIT 5`. The enumerate-and-sort floor alone, through
`/v1/sql` with no predicates: ~3 s warm for the winner set, ~2 s for the bidder set.

**Why bidder was a 503 and buyer 0.8 s.** The per-LOT org predicate `version_predicates` adds —
`EXISTS (… bp.tender_id = l.tender_id AND bp.seq = MAX(seq) AND bp.organization_id = ?)` — never
names the lot; it is a per-TENDER fact evaluated 137k times. And its seek: with the index set prod
carries, turso takes `tender_version_bid_parties_org_tender (organization_id=? AND tender_id=?)`,
i.e. org 357's rows for that tender across every version (~135), per lot — tens of millions of
index rows. `parties` has `_version (tender_id, seq)` and `result_winners` has its PK for the same
seek, which is why those two answered. Timed on prod through `/v1/sql`, bidder page: seed + sort
2.0 s, + `EXISTS(vl)` 2.3 s, + the per-lot bidder predicate → past the 10 s limit.

**Fix 1 (built): the seed IS the predicate, and the per-lot copy is gone.** `lot_seed_predicates`
now seeds `l.tender_id IN (SELECT s.tender_id FROM (SELECT DISTINCT tender_id FROM <table> WHERE
organization_id = ?[buyer role]) s WHERE EXISTS (SELECT 1 FROM <table> p WHERE p.tender_id =
s.tender_id AND p.seq = MAX(seq of s.tender_id) AND p.organization_id = ?<predicate role>))` — decided
at the head version, once per DISTINCT tender — and `lots_query` clears the seeded org from the
`Filter` it hands `version_predicates`. Two shapes that looked right and measured wrong on the way:
the head probe written per participation ROW (8–10 s: the correlated `MAX` re-evaluated for each of
194k rows), and the bidder role in the DISTINCT pre-level (4.2 s against 2.1 s — `_org_tender`
carries no `role`, so it was a table lookup per row). With the role only at the head level:
bidder page **4.8 s wall through `/v1/sql`, from a 30.6 s 503**; winner 5.4 s (its floor is below).
`the_org_seed_decides_membership_at_the_head_version_exactly` pins that a stale-version
participation and a subcontractor row do not leak now that nothing re-checks per lot;
`an_org_reverse_lookup_seeds_the_lots_stream_as_an_in_semi_join` pins the statement.

**Unit 1 had not landed on prod, and the reason is a second bug.** The journal at every boot:
`store: REFUSING to auto-build tender_version_result_winners_org_tender: … has ~845874165 rows,
over the 240000000 row cap`. `too_large_to_build` bounded by `MAX(rowid)`, and every full rebuild
deletes and re-inserts every satellite row, so after a dozen rebuilds the rowid space is an order
of magnitude past the row count — 845M for a table that cannot exceed `lot_results`' ~20M. The
covering index this issue's unit 1 recorded as LANDED 2026-09-16 was refused at every boot since;
`missing_deferred_indexes` queued an auto reindex after each deploy, the builder refused again in
0 s, and the loop repeated (jobs 1469 → 1477). The bid_parties twin built (its rowid space is
under the cap), which is why only the winner pre-seed still pays a per-row table lookup over org
357's 3.55M winner rows (~3 s of the 5.4 s). The refusal lines for `parties_org`, `_org_role`,
`classifications_code` are noise on top: those indexes exist (built index-first at a rebuild) and
the builder printed a refusal before checking.

**Fix 2 (built): `too_large_to_build` takes the cheap bound first and the truth second.** An
index that already exists is never refused (no line, no estimate); past the `MAX(rowid)` bound the
exact `COUNT(*)` decides — a scan, but on the background reindex path and only for tables whose
rowid space outgrew the cap. `a_high_rowid_alone_does_not_refuse_an_index_build` pins both halves.
After the deploy, the boot's auto reindex will build `tender_version_result_winners_org_tender`
for real (~20M rows sorted, well inside the 62 GB budget), and the winner pre-seed becomes
index-only.

**What stays open — the unit as this issue states it.** A page's cost still scales with the org's
lot count, not the page: the seed's lots are enumerated and sorted because `ORDER BY l.id` is an
order no seed can serve. Making it ∝ page means paging in an order the participation index CAN
serve — (tender_id, lot id), with a compound opaque cursor for the org-seeded lots stream — which is
a contract change for that one shape and wants its own decision. Not taken today; the numbers
above are its input.

## Comment — 2026-09-18: deployed `79c1bef`; the bidder page is 3.9 s from a 30 s 503, and the winners index is finally building

Live on prod right after the deploy, `/v1/lots?<seed>&limit=5`:

| request | before today | after the seed rewrite (`9d41dc8`) | now (`79c1bef`) |
| --- | --- | --- | --- |
| `bidder=357` | `503` 30.6 s | `503` 30.6 s | **`200` 3.9 s**, 5 items |
| `winner=357` | `503` 30.5 s cold / — | `200` 22.5 s cold, 6.1 s warm | `200` 5.8 s (pre-seed still uncovered, see below) |
| `buyer=357` / `winner=388` (controls) | 0.8 s / 0.5 s | same | 0.6 s / 0.4 s |

And the boot did what the fixed cap lets it: `supervisor: 1 deferred index(es) missing
(tender_version_result_winners_org_tender) — queueing a reindex AHEAD of 0 pending job(s)`, then
the reindex RAN instead of refusing in 0 s — no `REFUSING` line, the exact `COUNT(*)` passed, and
`CREATE INDEX` is sorting the winners table as this is written. When it lands, the winner pre-seed
(`SELECT DISTINCT tender_id … WHERE organization_id = ?`) goes index-only and the ~3 s of per-row
table lookups over org 357's 3.55M winner rows drop out of that 5.8 s; the number goes here.

The gate bit once on the way, and it was CLAUDE.md's documented trap: the two extra queries in
`too_large_to_build` grew the three index builders' futures, which the `Reindex` arm awaits inside
`run_spec`'s one giant future, and `an_execute_without_an_expected_count_is_refused` — a test with no
relation to any of this — overflowed its stack. Boxing the arm's body (`Box::pin(async move …)`) is
the fix the note prescribes, and it held (117 suites, 0 overflows).

## Comment — 2026-09-18 (later): the winners index is built; the final numbers, and what the unit still owes

Job **1478**, `reindex auto: tender_version_result_winners_org_tender`, `ok`, **281 s** — the first
time that job has done anything but refuse in 0 s. Read afterwards, warm: `?winner=357` **3.0 s**
(5.8 s before the index), `?bidder=357` **3.7–3.8 s**, `?buyer=357` 0.6 s, `?winner=388` 0.4 s. One
`bidder=357` read in the minute after the sort: 20.6 s — a cold page cache over 137k lots, the exact
residual this issue is about.

So, against `## Done when`: the 30 s → 503 class is dead for the most prolific org in the corpus
(unit 1's index is finally real, and the per-lot predicate is gone); the "cost scales with the page"
clause is NOT met — warm cost is the org's lot count enumerated and sorted, cold cost is that
count's I/O. The design that meets it is recorded above (page the org-seeded lots stream in
`(tender_id, lot id)` order with a compound opaque cursor, so the participation index serves the
order and a page reads a page), and it is a contract decision for that one shape, taken separately.
Issue 223 is closed on today's numbers; this issue stays open on that clause.

## Comment — 2026-09-18 (boot check): the loop is closed

The `2df1a49` boot (the first since job 1478 built the winners index) logged no "deferred index(es)
missing" line and queued no reindex — the first boot since 2026-09-16 to do neither — and printed no
`REFUSING` line for the indexes that exist. The estimator's two-day loop is over; `## Verify`-class
read for it: `journalctl -u tender-db -b | grep -c REFUSING` → `0`.

## Comment — 2026-09-18: the cursor inside the seed — BUILT; a page now costs a page

**The decision, taken.** The "contract change for that one shape" the previous comments deferred:
the organization-seeded lots stream (`/v1/lots?winner=` and `?bidder=`, `participation_seed`'s
two covering-index shapes, with no `tender=`) no longer pages in lot-id order. It pages in
**`(tender_id, lot id)` order** — the order `tender_version_result_winners_org_tender` /
`tender_version_bid_parties_org_tender` serve — and its `next_cursor` is **compound**,
`<tender_id>:<lot_id>`. Within the documented contract: `/docs` and `openapi.json` have always
said the cursor is opaque, pass it back verbatim, and is specific to its query shape. A bare-id
cursor sent to this shape (a client mid-walk across the deploy) parses as nothing and restarts the
walk from page one, which is what every list already does with an unparseable cursor; the docs'
pagination paragraph and the "ascending id everywhere" sentence now name the exception. The buyer
seed stays on the id-ordered stream: its index is `(organization_id, role, tender_id)` behind a
`LIKE` on role, which serves no order, and its cost was never the problem (org 357 as buyer 0.6 s).

**The mechanism (`store::read::lots_seeded_page`).** Per page, up to `DEFAULT_SEED_WINDOWS_PER_PAGE`
(8) windows of:

1. one index range read — `SELECT tender_id FROM <table> WHERE organization_id = ? AND tender_id >= ?
   ORDER BY tender_id LIMIT 256` — the walk's order AND its bound in one statement; the plan, pinned by
   `the_seed_window_is_an_index_range_read_in_tender_order`, is `SEARCH … USING INDEX …_org_tender
   (organization_id=? AND tender_id>=?)`, no scan, no sorter. The window is in index ROWS, not
   tenders, because a tender the org bid on across 135 versions has 135 rows there;
2. membership at the HEAD version once per candidate tender (`head_members`: one seek each, the
   seed IS the predicate — the first unit's rule, kept), never per lot;
3. those tenders' lots after the cursor, through the SAME stream head and predicates the id-ordered
   page uses (`lots_stream_head` / `lots_stream_predicates`, factored out of `lots_query_banded` so
   the two shapes cannot disagree about what a matching lot is), `ORDER BY l.tender_id, l.id LIMIT
   room` — a sorter over one window's lots, not the org's.

A full page's cursor is the last row returned; a page that runs out of windows comes back short with
the cursor past the last tender examined (the 408 (b) contract, unchanged: page length is not an
end signal, `more` is); `next: null` only when the seed ran out inside the call.

**Tests.** Store (`crates/store/tests/seeded_lots_page.rs`): a nine-tender fixture where
`(tender, lot)` order is visibly not lot-id order, versions per tender vary so 2-row windows end
inside tenders, a stale-version win, a subcontractor-only bid and a lotless winning tender all sit
in the seed — the walk returns exactly the id-ordered stream's SET (`read::lots`, the oracle) in
`(tender, lot)` order once each and terminates, at window 2 / one window per page (many short pages)
and at the production window; a companion `kind=Part` admits nothing and still pages short-with-
cursor, not a corpus pass; the cursor round-trips and rejects a bare id. Handler
(`an_org_seeded_lots_walk_pages_in_tender_order_with_a_compound_cursor`): on the chain fixture the
`limit=1` walk reproduces the one-page answer row for row with a compound cursor on every page, a
bare-id cursor restarts, and `/v1/tenders?winner=` keeps its bare-id cursor. The bounded-walk,
lots-kind and winner-semantics suites still pass beside it.

**What the handler gained on the way.** `read_page` now owns the whole page contract — the `+1`
probe, the truncation, the examined-to cursor and the seeded arm — and returns `PageOut {items,
next}`; the handler passes the raw cursor string in and the rendered cursor out, so no cursor
grammar is parsed anywhere but beside the read that defines it (`Params::after` is gone).

**Read live, 2026-09-18 12:14 UTC, rev `c283dd8`, warm, `limit=100`, network-inclusive:**

| request | before (`79c1bef`, 09-18 morning) | now |
| --- | --- | --- |
| `/v1/lots?bidder=357` page 1 | 3.7–3.8 s warm, **20.6 s** cold | **0.62–0.74 s**, 100 items, cursor `2184:5723` |
| `/v1/lots?winner=357` page 1 | 3.0 s warm, 5.8 s before the index | **0.89 s**, 100 items |
| `/v1/lots?bidder=357` pages 2–10 (ten cursors followed) | — (each page cost the org) | **0.52–0.75 s each**, 100 items each |
| `/v1/lots?buyer=357` / `?winner=388` (controls) | 0.6 s / 0.4 s | 0.53 s / 0.44 s |

A page costs a page: the tenth page of the walk (`cursor=5285:14690`) is no dearer than the first,
which is the clause this issue was filed on. The boot logged no deferred-index or REFUSING line.

**Still open — the tenders shape.** `/v1/tenders?bidder=357&limit=100` read **3.15 s** and
`?winner=357` 1.25 s in the same minute: `tender_from`'s seed is still `(SELECT DISTINCT tender_id
FROM <table> WHERE organization_id = ?) hits JOIN tenders t … ORDER BY t.id LIMIT`, the org's whole
index slice DISTINCTed per page. The `## Done when` asks under 1 s there too. The walk above fixes
it with no contract change at all — tenders already page in `tender_id` order, so the window IS
the page order and the cursor stays a bare id. Next unit.

## Comment — 2026-09-18 (later): the tenders half — the same walk, no cursor change

`tenders_page` routes `seeded_tenders(filter)` — the winner/bidder seeds with no publication seed
— to `tenders_seeded_page`: the same `(organization_id, tender_id)` window (`tender_id > cursor`,
the plan pinned for both bounds), the same `head_members`, then the paged shape's OWN predicates
(`tender_page_predicates`, factored out of `tenders_page_query` so the two cannot drift) over
`t.id IN (members) AND t.id > cursor ORDER BY t.id LIMIT room`, wrapped by the same satellite
SELECT (`tender_page_wrap`). Tenders already page in `tender_id` order, so the window IS the page
order: a full page's cursor is its last row, a short page's is the last tender examined
(`examined_to`), `None` means the seed ran out — the existing `Banded` contract, and the handler
did not change at all. The seeded org's per-tender EXISTS is dropped (`without_seeded_org`, now
shared with the lots walk), because `head_members` decided it.

Pinned: `the_seeded_tenders_walk_returns_the_stream_set_in_id_order_once_and_terminates` — the
id-ordered stream's set (`read::tenders`, the oracle; a lotless winning tender IS a tender here) at
2-row windows and at the production window, routed through `tenders_page`, a guard-answered
companion (`source=doe`) and a walk-answered one (`published_after` in 2033: short pages with a
cursor, then the end); and the handler walk at `limit=1` over `/v1/tenders?winner=` beside the
lots one.

**Deployed at `5be4c5c` 12:30 UTC and read at 12:31 — one number went the WRONG way, and the
walk's first cut had two defects the fixture was too small to price.** `/v1/tenders?bidder=357`
page 1: 0.73 s from 3.15 s, good; but `/v1/tenders?winner=357` page 1: **3.44 s from 1.25 s**,
and both pages came back SHORT (25 and 26 rows at `limit=100`). Read off the plans, same
afternoon:

1. `head_members` drove from the participation table with `p.tender_id IN (…)` and a correlated
   `MAX(seq)`, and turso bound ONLY `organization_id` — `SEARCH … USING INDEX …_org_tender
   (organization_id=?)`, a hash table for DISTINCT, the subquery per row. Every call scanned the
   org's whole slice: 3.55M rows for org 357's wins, eight times per page. Now it drives from
   `tenders` by primary key with the row's own `current_seq` as the head, and the probe is one
   seek per candidate (`SEARCH p USING INDEX sqlite_autoindex_…_1 (tender_id=? AND seq=?)` for
   winners, `…_org_tender (organization_id=? AND tender_id=?)` for bids).
2. The window counted index ROWS (256), and org 357 carries ~950 winner rows per tender, so
   eight windows were ~26 tenders and the page was short by construction. The window is now in
   TENDERS: `GROUP BY tender_id ORDER BY tender_id LIMIT 64` — turso serves the grouping off the
   index's own order with no temporary structure and stops at the limit (`DISTINCT` builds a
   hash table over every row past the cursor instead; both plans read). The plan probe test pins
   the grouped statement.

The lots walk gained both fixes too; its first-cut numbers above stood only because a page of
100 lots spans a handful of org-357 tenders. Numbers after the second deploy below.

## Verify

    for u in 'lots?bidder=357&limit=100' 'tenders?bidder=357&limit=100'; do curl -s -o /tmp/v.json -w "$u %{http_code} %{time_total}s " --max-time 40 "https://tenders.zebreus.click/v1/$u"; python3 -c "import json; print(len(json.load(open('/tmp/v.json'))['items']))"; done

- **done**: both `200`, both well under 3 s warm, both `100` — a page of the most prolific org costs a page
- **open**: either over ~3 s, a `503`, or fewer than 100 items (read 2026-09-18 13:14 at `62b3ce5`: `200 0.71s 100` and `200 0.89s 100`)

## Closing matrix — 2026-09-18 13:14 UTC, rev `62b3ce5`, warm, network-inclusive

Three deploys today, each read live and corrected the same afternoon: `c283dd8` (the lots walk),
`5be4c5c` (the tenders half — read at 12:31 with `?winner=357` at 3.4 s and pages of 26, the two
defects above), `2083c63` (the head check seeks per candidate; the window is in tenders),
`4c93965` (the window budget follows the page), `62b3ce5` (the window itself follows the page; the
plan probe builds from the index recipe).

**A page costs a page.** `limit` 1 / 100 / 1000: lots `?bidder=357` **0.42 / 0.71 / 0.92 s**;
tenders `?bidder=357` **0.42 / 0.89 / 3.18 s** (the 1000-row tenders page is 1000 tenders'
satellites, 558 kB; the unseeded `?limit=1000` is 0.96 s over CONTIGUOUS ids 1–1029, the seeded one
spans ids 1,724–823,196). A one-row page reads a four-tender window now; at the fixed 64-tender
window it cost 0.92 s, more than a hundred rows.

**The `## Done when` reads, clause by clause:**

| clause | reading |
| --- | --- |
| covering indexes exist, built, index-only plans | unit 1 + job 1478 (2026-09-18); `the_seed_window_is_an_index_range_read_in_tender_order` pins `SEARCH … USING INDEX …_org_tender (organization_id=? AND tender_id>?)`, no scan, no sorter, for both tables and both bounds — and the fixture builds its indexes through `Db::build_tender_indexes`, the recipe itself |
| cursor inside the seed; `limit=1` cheaper than `limit=1000`; the deep page no dearer than page 1 | 0.42 s vs 0.92 s (lots), 0.42 s vs 3.18 s (tenders). Deep page: `?bidder=357&cursor=859213` **1.39 s warm** (4.5 s cold) against page 1's 0.88 s, alternated three times. The walk is not the difference — the four-candidate head check at that cursor reads 13 ms on the box, the grouped window 37–59 ms with 1.35M of the org's rows past it (a scan would be ≥109 ms, the `COUNT`); the difference is the SATELLITES of those particular hundred tenders (an unseeded hundred in the same id region: 0.63 s, contiguous). Read as: the org-dependent cost is gone, the tender-dependent cost is every page's |
| the five reads under 1 s warm at `limit=100`, cold first under 3 s | `?bidder=357` **0.90**, `?winner=357` **0.74**, `?bidder=355` **1.10**, `?winner=355` **0.80**, `/v1/lots?bidder=357` **0.71 s**. Four of five; the fifth at the line, and its unseeded twin costs 0.9–1.0 s. Cold first reads after each deploy today: 0.73–0.89 s |
| controls unmoved; `winner=357&limit=1000` byte-identical | `bidder=23494787` 0.44 s (≤0.42), `bidder=28` 0.43 s (≤0.43), `buyer=28&limit=1000` 0.50 s (≤0.89). The 1000-row page is **557,774 B** against 556,138 B on 09-13 — five daily ingests apart, so identity cannot be asserted; what was checked instead: its 1000 ids are strictly increasing and unique, and on the first hundred the served set EQUALS the org's index superset up to the cursor (100 = 100, `/v1/sql`), while `current_seq = MAX(seq)` on every tender in that range (drift 0 of 75,810) |
| a gate pins the class | the plan probe over the recipe-built fixture, above |
| 223's status line and `participation_seed`'s comment corrected | 223 closed on today's numbers; the comment now says what the seeds bound (the org's ROWS — 3.26M bid rows, 3.55M win rows for org 357 — not its tenders) and that the pages walk instead |

**What still reads the seed whole.** The SSE snapshot for an org-seeded subscription
(`read_items`, id order) — one pass per subscription rather than per page, and filed as its own
issue (416) rather than folded in here. The `buyer` seed is untouched on both streams (0.5 s on
the controls; its index serves no order).

