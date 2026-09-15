# 223 — org reverse-lookups (buyer/winner/bidder) walk all tenders for a PRESENT value → 35 s+ client timeout

Status: REOPENED 2026-09-15 — the fix was only ever verified on `/v1/tenders`; the `lots_query` half of
the same participation seed still 503s at the 30 s service bound on prod rev `9e082fd` (see the
2026-09-15 comment below). Incomplete fix, not a regression: the lots path was never measured.

Previous status (history, 2026-08-16):

Status: RESOLVED for `winner`/`bidder`, PARTIAL for `buyer` — DEPLOYED & VERIFIED 2026-08-16 (serving
rev `f3a1628`). The driven set is now seeded from the participation table's `organization_id` index
(`participation_seed` + a `hits` subquery in `tenders_query`/`lots_query`); the untouched `EXISTS`
predicates still decide membership, so results are byte-identical to the walk. **Measured on prod:** the
org that timed a 35 s client out last firing (`23494787`) now returns in **10 ms** for `bidder=` and
**2.6 ms** for `winner=`; across a spread of orgs, every `winner=`/`bidder=` lookup is sub-25 ms. One
residual: **`buyer=` still walks for a handful of ubiquitous non-buyer PARTY orgs** (e.g. org 2/3:
~17–19 s) — see the residual section below — split out as issue **225**. No regression: those orgs
walked before too, and the isolated pool kept the main pool at 0.03 s throughout.

Was: needs-triage — HIGH (usability; the reverse-lookups are advertised filters that time out on
their most common input), CONFIRMED (prod-measured) 2026-08-16. Filed by the owner while shipping the
`bidder` filter (issue 217-C): every org reverse-lookup returns fast for an ABSENT org (short-circuit,
issue 219) but times out a 35 s client for a PRESENT one.

## Residual: `buyer=` for ubiquitous non-buyer party orgs (→ issue 225)

`winner` and `bidder` seed from `tender_version_result_winners` / `tender_version_bid_parties`, where an
org's row count is bounded by how often it won / bid — small, so the seed is cheap and every tested org
is sub-25 ms. `buyer` seeds from `tender_version_parties`, which holds **every** tender-level party role,
not just buyers. A few orgs (org 2, org 3) are parties on a huge fraction of the corpus in some
NON-buyer role, so `SELECT DISTINCT tender_id FROM tender_version_parties WHERE organization_id = ?`
returns a massive candidate set that the `%Buyer%` EXISTS then filters to ~0 — the work is paid before
the filter. Because `tender_version_parties_org` covers only `organization_id`, the seed also does one
table lookup per row to read `tender_id`, so it cannot even be answered index-only. The fix is a covering
index (`tender_version_parties(organization_id, tender_id)`, ideally `(organization_id, role, tender_id)`
so the seed filters to buyer rows index-only) via the deferred-index builder (issue 111), plus adding the
role predicate to the buyer seed. That is index-infra + a reindex job, so it is its own issue (**225**),
not a rushed add here.

Kind: performance / usability (present-value walk on the participation reverse-lookups)
Blocked by: —
Relates to: 117 (this is a Class-B instance — the per-row `EXISTS` version predicate driven off the
`tenders` PK by `ORDER BY id LIMIT` pagination), 120 (the isolated pool it walks, so it sheds rather
than starving the main pool), 219 (the absent-value short-circuit that already covers the cheap half),
217 (the reverse-lookup filters themselves — `buyer`/`winner`/`bidder`)

## Defect

`version_predicates` (`crates/store/src/read.rs:600-643`) implements `buyer`, `winner`, and `bidder`
as per-row `EXISTS` subqueries over `tender_version_parties` / `_result_winners` / `_bid_parties`:

```rust
if let Some(winner) = f.winner {
    q.push(" AND EXISTS (SELECT 1 FROM tender_version_result_winners w
                  WHERE w.tender_id = {tid} AND w.seq = {seq} AND w.organization_id = ?)", …);
}
```

The list path pages with `AND t.id > ? ORDER BY t.id LIMIT ?` (`read.rs:988`), which forces the planner
to **drive from `tenders` by its id PK** and evaluate the `EXISTS` per driven row. No index on the
participation tables can help, because the query is not driven from them — it is driven from `tenders`
and filtered. For a PRESENT org the planner still has to walk id-ordered tenders until it fills a page,
and a given org appears on a vanishingly small fraction of the ~4.26M tenders, so it walks essentially
the whole corpus. This is exactly the issue-117 Class-B shape.

`reachable()` (`read.rs:716-760`) already probes `buyer`/`winner`/`bidder` (issue 219), so an **absent**
org short-circuits to an empty page in ~ms. The walk only bites for the PRESENT case — the case the
filter exists to serve.

## Measured (prod, serving rev `00f2f48`, 2026-08-16)

| request | result |
|---|---|
| `/v1/tenders?winner=<present org>` | **HTTP 000 — client timed out at 35 s** |
| `/v1/tenders?bidder=<present org>` | **HTTP 000 — client timed out at 35 s** |
| `/v1/tenders?winner=<absent org>` | 200 empty, fast (short-circuit) |
| `/v1/tenders?bidder=<absent org>` | 200 empty, fast (short-circuit) |

`buyer` shares the identical predicate shape and drives off the same PK pagination, so it walks too — it
was simply not re-measured this pass. All three participation reverse-lookups are affected identically.
The walk runs on the isolated pool (`walks()` routes it there, `read.rs:508`), so it sheds `503` under
saturation rather than starving the 8 main REST readers — the availability blast radius is bounded by
issue 120. What is NOT bounded is the single-request latency: a legitimate `?winner=<real org>` never
returns to the caller.

## Fix direction

Drive the query from the participation table's org index instead of from the `tenders` PK. The
reverse-lookup's natural driver is "the set of `tender_id`s where org X participated", which the
participation tables *can* serve from an index on `organization_id` — the id set is small (an org
appears on few tenders), so seeking it and then joining to the current tender rows is cheap. Concretely,
when a participation filter (`buyer`/`winner`/`bidder`) is present and no companion predicate forces the
tenders-driven shape, rewrite the query to:

1. seek the participation table by `organization_id` (add the index if absent — the deferred-index
   builder, issue 111) to get the candidate `tender_id`s,
2. join to `tenders` / current-version rows and page **that** set,

so the work scales with the org's participation count (tens–thousands of rows) rather than with the
whole corpus. This is the same "drive from the selective side" correction issue 16 made for lots'
containment probe. Pagination has to move to the driven set's key; keep the existing tenders-driven path
for the companion-filter case (or intersect), and keep the isolation routing either way.

Note this is the participation twin of the `publication_id` fast-path follow-up in issue 217 (both are
"seek the selective index instead of walking id-ordered tenders"); they can share the query-shape work.

## Verification

- `EXPLAIN QUERY PLAN` for the current `winner=?` list query shows `SCAN tenders` driving the `EXISTS`
  (establishes the walk without running it on prod).
- After the fix: `/v1/tenders?winner=<present org>` and `?bidder=<present org>` return the matching page
  in well under a second; the absent-value short-circuit still returns fast; and a companion filter
  (`?winner=X&country=DE`) still returns correct results.

## Comments

### 2026-09-15 — API/data-quality review fan-out: incomplete fix — the `lots_query` half was never verified; `/v1/lots?winner=<org>` still 503s at the 30 s bound

This issue was closed RESOLVED-VERIFIED for `winner`/`bidder` on the strength of a probe table that
measured **only `/v1/tenders`** (lines 71-76 above, and the "sub-25 ms" claim in the old status). The
status text asserts the seed went into `tenders_query`/`lots_query` both, with "results byte-identical
to the walk" — but the lots half was never measured on prod, and it does not hold. On serving rev
`9e082fd1893156446eb50dc880a5d2052f21c532` a documented filter on a documented collection fails 100% of
the time for its intended input.

Evidence (literal, one request at a time, outside the ingest window; rev confirmed via `GET /health`):

```
curl -sS -o /dev/null -w '%{http_code} %{time_total}s\n' 'https://tenders.zebreus.click/v1/lots?winner=14189&limit=5'
  -> 503 30.394811s, body {"error":{"message":"no response within the 30s service bound — a stalled internal wait, not your request; safe to retry","status":503}}
curl -sS -o /dev/null -w '%{http_code} %{time_total}s\n' 'https://tenders.zebreus.click/v1/lots?winner=5550&limit=5'
  -> 503 30.659685s
curl -sS -o /dev/null -w '%{http_code} %{time_total}s\n' 'https://tenders.zebreus.click/v1/tenders?winner=14189&limit=5'
  -> 200 0.773681s (5 tenders, ids 4, 1067, 11416, 16554, 20808)

ssh root@zebreus.click 'echo "SELECT count(*) AS winner_rows, count(DISTINCT tender_id) AS tenders FROM tender_version_result_winners WHERE organization_id = 14189" | /root/sq.sh' -> [1300, 306]
ssh root@zebreus.click 'echo "SELECT count(*) AS winner_rows, count(DISTINCT tender_id) AS tenders FROM tender_version_result_winners WHERE organization_id = 5550"  | /root/sq.sh' -> [280, 127]
```

Latency scales with the number of DISTINCT won tenders in the seed (~0.8-1.1 s per hit tender), not with
winner-row count and not with lot-id walk depth (org 14189's first won tender, id 4, owns lots 4-101 at
the very start of the table, yet still stalls). Every hit tender appears to pay roughly a full pass over
`lots` (max id 14,127,281):

| org (`winner=`) | distinct won tenders | winner rows | `/v1/lots?winner=…&limit=5` | `/v1/tenders?winner=…&limit=5` |
|---|---|---|---|---|
| 14182 | 1 | 12 | 200 @ 2.097 s | — |
| 14200 | 7 | 1,241 | 200 @ 7.747 s | — |
| 14187 | 10 | 22 | 200 @ 11.308 s | — |
| 14171 | 21 | 21 | 200 @ 17.616 s | — |
| 14259 | 41 | 50 | **503 @ 30.739 s** | — |
| 5550 (ALPINUS CHEMIA) | 127 | 280 | **503 @ 30.660 s** (judge re-run: 503 @ 30.51 s) | 200 @ 0.724 s (judge: 0.79 s) |
| 14189 (Nobipharm) | 306 | 1,300 | **503 @ 30.395 s** (repro: 503 @ 30.455 s) | 200 @ 0.774 s (repro: 0.872 s) |

The shed line sits between 21 and 41 won tenders (~28-35 at ~1 s each). Prevalence in one id band —
`SELECT sum(t > 28), sum(t > 10), count(*) FROM (SELECT organization_id, count(DISTINCT tender_id) AS t
FROM tender_version_result_winners WHERE organization_id BETWEEN 14000 AND 15000 GROUP BY organization_id)`
-> `[111, 185, 491]`:

| org ids 14000-15000 | winner orgs | share |
|---|---|---|
| > 28 won tenders (503 at the bound) | 111 / 491 | 23% |
| > 10 won tenders (>= ~10 s) | 185 / 491 | 38% |

Even a one-tender winner pays ~2 s on `/v1/lots` against <1 s on `/v1/tenders`. `bidder=` and `buyer=`
share `lot_from` and are expected to behave the same; they were not probed, since each hold pins an
isolated slot for 30 s.

Judge's reasoning for why this is ours and belongs here: *Premise verified live and in code. (1) Re-ran
the repro once at 00:53 UTC 2026-09-14 (outside the ingest window), served rev `9e082fd` (GET /health):
`curl 'https://tenders.zebreus.click/v1/lots?winner=5550&limit=5'` -> 503 in 30.51 s with the "no
response within the 30s service bound" body; `curl
'https://tenders.zebreus.click/v1/tenders?winner=5550&limit=5'` -> 200 in 0.79 s. With the reviewer's two
probes that is 3 of 3 present-winner lookups on `/v1/lots` shed at the bound, 2 of 2 on `/v1/tenders`
fast. (2) The mechanism is this system's own query shape, not anything the publisher published:
`crates/store/src/read.rs` `lot_from()` (line 2549) still emits the participation seed as `(SELECT
DISTINCT tender_id FROM tender_version_result_winners WHERE organization_id = ?) hits JOIN lots l ON
l.tender_id = hits.tender_id`, and `lot_seed_predicates()` (line 2584) returns early whenever a
participation seed is present. Issue 275 (RESOLVED, deployed `0fe1d64`) measured exactly this JOIN form
inverting on lots — "turso drives from `lots` to serve ORDER BY l.id and probes `hits` per row" (CY 2.1 s
as JOIN vs 0.32 s as IN) — and rewrote only the country/status arms as `l.tender_id IN (...)`, explicitly
leaving the participation seed in the JOIN form ("participation seed still outranks"); the doc comment on
`lot_from` even records the inversion while keeping the JOIN. `git log 9e082fd..HEAD --
crates/store/src/read.rs` is empty, so HEAD (`5c47984`) has the same shape. (3) Not resolved on the
board: issue 223 is marked RESOLVED for winner/bidder and issue 225 says "every participation
reverse-lookup ... is sub-second for every org class tested", but every measurement in both issues is on
`/v1/tenders`; 223 asserts `lots_query` was seeded too yet never measured it, and the JOIN seed that
works on tenders (joins on the outer table's PK `t.id`) is the form 275 later proved inverts on lots
(`l.tender_id` is not the lots PK). So the resolution claim does not hold on `/v1/lots` — this is the
un-verified half of 223, exposed by the same finding 275 made for country. No OPEN issue covers it (grep
of the board for `lot_from` / `lots?winner` / participation seed hits only 217, 223, 225, 275, all
RESOLVED/DONE). Also worth noting: the /docs performance table (`crates/app/src/v1/docs.rs` line 512)
still lists `?winner=<rare>` as "walks -> up to a full scan; 503 under load", a row 223 should have
retired for tenders and which is now accidentally true only for lots — a docs/behaviour inconsistency the
fix should clear. (4) Actionable and cheap: mirror 275's rewrite — `FROM lots l ... AND l.tender_id IN
(SELECT DISTINCT tender_id FROM <participation table> WHERE organization_id = ? [AND role LIKE
'%Buyer%'])` — for winner/bidder/buyer in `lot_seed_predicates`, drop the JOIN arm from `lot_from`, add a
statement-pin test beside `crates/store/tests/lots_country_seed.rs`, verify on prod for all three
filters, and fix the stale docs row. Severity high by the board's own precedent: 223 rated the tenders
twin HIGH ("advertised filters that time out on their most common input") and 275 rated a lots 503 as
issue-61 class (unauthenticated, trivially reachable); here a documented filter on a documented
collection (docs line 184, OpenAPI `winner` on `/v1/lots`) fails 100% of the time for its intended input
and pins an isolated slot for 30 s per request, though the blast radius is bounded to the isolated pool.*

To close: rewrite the winner/bidder/buyer participation seed in `lot_seed_predicates()` as `l.tender_id
IN (SELECT DISTINCT tender_id FROM <participation table> WHERE organization_id = ? [AND role LIKE
'%Buyer%'])` and drop the JOIN arm from `lot_from()` (mirroring issue 275's country fix), pin the
statement in a test beside `crates/store/tests/lots_country_seed.rs`, then re-measure `/v1/lots` for all
three filters on prod — the same table this issue already has for `/v1/tenders` — and retire the stale
`?winner=<rare>` row in `crates/app/src/v1/docs.rs:512`.
