# 388 — a reverse lookup by a prolific bidder/winner org re-pays the org's whole participation set on every page: 2.3–4.7 s warm, 15–28 s cold, against a documented sub-25 ms contract

Status: ready-for-agent — unit 1 (the two covering indexes) LANDED 2026-09-16, see the foot; the cursor-inside-the-seed rewrite and the prod re-measurement are open. Filed 2026-09-15 by the API/data-quality review fan-out (32 lenses, every finding independently reproduced and adversarially judged)
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
