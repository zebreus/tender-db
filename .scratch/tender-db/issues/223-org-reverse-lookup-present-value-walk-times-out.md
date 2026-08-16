# 223 — org reverse-lookups (buyer/winner/bidder) walk all tenders for a PRESENT value → 35 s+ client timeout

Status: needs-triage — HIGH (usability; the reverse-lookups are advertised filters that time out on
their most common input), CONFIRMED (prod-measured) 2026-08-16. Filed by the owner while shipping the
`bidder` filter (issue 217-C): every org reverse-lookup returns fast for an ABSENT org (short-circuit,
issue 219) but times out a 35 s client for a PRESENT one.

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
