# 371 — reachable() and walks() drifted apart again: ?currency=XXX holds an isolated slot for 29.8 s and answers empty

Status: DONE 2026-09-07 (owner) — built (`50f6749`, 920 passed), deployed, and the present-set backfilled and attested on prod (job 800). `?currency=XXX` went from 30 s + 503 to **200 in 20 ms**. Unit 4 stays with issue 273 (a present-but-rare value still walks, by construction). Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings)
Kind: defect (read layer — filter admission), availability
Relates to: 219 (fixed exactly this class for country/cpv/buyer/winner/kind, and STATED
the invariant this violates), 273 (the present-but-rare walk; step 2 deliberately
deferred — not this issue), 238/239/120 (the same "admitted, then unstoppable" family on
/v1/sql)

## Observed (timed live against prod, rev `d80a4bd`)

- `/v1/tenders?currency=XXX&limit=2` → 200 **EMPTY after 29.78 s** (just inside `REQUEST_DEADLINE`, `crates/app/src/v1/mod.rs:408`; under any contention this is a 503)
- `/v1/tenders?currency=DEM&limit=2` → 200 after 6.68 s, first row id 1,844,394 (`ojs:1992-033318`)
- `/v1/tenders?currency=EUR&limit=2` → 200 after 0.65 s, first row id 1
- The gradient is the id-position of the first match, not the number of matches: EUR at id 1 = 0.65 s, DEM at 1.84M = 6.7 s, XXX never = 29.8 s over the full 7.93M rows.
- The docs promise the opposite: "Filter, absent value | `?country=ZZ` | <1 ms*" with "* an absent filter value short-circuits to an empty page" (`crates/app/src/v1/docs.rs:511`, :517). No caveat names currency.
- A `COUNT(*)` over three `LIMIT 5` currency subqueries through `/v1/sql` returned `{"error":{"message":"query exceeded the 10s time limit","status":408}}`.
- Grep of all 341 issue files: no currency short-circuit anywhere. Issue 219's own table enumerates the guard legs it added (country / cpv / buyer / winner + kind); currency is in neither the issue nor the shipped function.

## Why, exactly

- **The two sets drifted.** `walks()` declares `currency` a version predicate that must run isolated — `let version_predicate = country.is_some() || … || currency.is_some();`, `crates/store/src/read.rs:657-665` — while `reachable()` (`crates/store/src/read.rs:994-1101`) probes country(nuts), cpv, buyer, winner, bidder, source (lots only) and kind, and has **no currency leg**. So a value present nowhere is ADMITTED and pays the full density-bounded walk while holding one of `const SLOTS: usize = 4` (`crates/app/src/v1/isolate.rs:105`) for ~30 s. The pool is global and unauthenticated (isolate.rs:100-104), so four such requests shed everything else on those endpoints.
- **The invariant was written down and nothing enforces it.** Issue 219's stated fix direction was to fold all of it into one `reachable()`-style probe covering every isolation-routed filter "so the guard set and the `walks()` set cannot drift again". There is no test and no compile-time coupling between the two sets, so adding `currency` to `walks()` silently reopened the hole.
- **A guard leg would have nothing to seek.** The only index on the table is `tender_version_amounts_version ON tender_version_amounts(tender_id, seq)` (`crates/store/src/canonical.rs:283`) — `currency` is unindexed, so an existence probe is a bare table pass, unlike the country/cpv legs which seek `tender_version_classifications(scheme, code)`.

## Units

1. **Couple the two sets in the type system, not in prose**: make the isolation-routed filters a single enumeration each filter registers in, with the guard leg a required member — an unimplemented leg is a compile error, not a silent walk. This is 219's own stated fix, and its absence is why this recurred.
2. **Give the currency leg something seekable**: either an index on `tender_version_amounts(currency)` (measure the fold's write cost first) or a small currency dimension the projection maintains. Measure, decide, record here.
3. Correct `crates/app/src/v1/docs.rs:511`/:517 to name which filters short-circuit (row in `served-claims-nothing-re-derives`).
4. **Not this issue, for the board**: the present-but-rare walk (GR first match at id 2,170,693 → 19.2 s; DEM → 6.7 s) is issue 273's deferred step 2 — `reachable()` "answers matches-nothing only; a present-but-rare source still walks" by construction (`crates/store/src/read.rs:1053`).

## Decisions (2026-09-07, owner)

**Unit 2 — a present-set, not an index.** The probe asks one question, "does any row carry this
value", over a column with a few dozen distinct values. An index on
`tender_version_amounts(currency)` would answer it by paying write amplification on every fold
over tens of millions of rows to store a key whose cardinality is tiny. Instead the projection
maintains a small present-set of the currencies it has written, and `reachable()` seeks that. The
failure mode is the safe one: an entry left behind after the last row carrying it disappears makes
the probe ADMIT, which degrades to exactly today's walk rather than to a wrong answer — so the set
may be a superset, and never has to be exact.

**Unit 1 — the compiler holds the coupling.** Make the isolation-routed filters one enumeration
and give `reachable()` an exhaustive match over it, so a filter added to `walks()` without a
guard leg does not compile. Prose asked for this in issue 219 and prose is what failed; the
point is to make the next drift impossible rather than to fix this one instance.

## Done when

- `?currency=XXX` answers empty in <1 ms;
- a new isolation-routed filter cannot be added without a guard leg, and a test or a compile error proves it;
- the `/docs` performance table matches the code.

*One issue because:* it is one filter, one missing probe leg, and the general failure is the coupling 219 asked for and nobody built — which is what makes it worth an issue rather than a one-line patch.

## Done (2026-09-07)

**Built** as decided: `isolation_routed()` names each isolation-routed filter, `walks()` is "that
list is non-empty", and `reachable()` matches over it EXHAUSTIVELY — a filter routed to isolation
without a guard leg does not compile, and the error text is quoted in the type's docs since a
future reader cannot run the negative case. The honest limit is recorded too: the match forces a
DECISION, not a correct one (five arms decline deliberately, each with its reason).

The currency leg seeks a projection-maintained present-set, written in the same transaction as
the amount rows, so it can never be observed as a subset; nothing deletes from it, so it is a
superset and a stale entry degrades to a walk. The fatal direction is guarded twice: a corpus
whose amounts predate the table has `currency_presence_complete = 0` and the guard declines
entirely, and a rebuild clears the set and refills it fact by fact.

**Measured on prod after `backfill-currencies` (job 800, swept 7,929,584 tenders and attested):**

| request | before | after |
|---|---|---|
| `?currency=XXX` (absent) | 30 s, **503** | **200 in 20 ms** |
| `?currency=EUR` (present) | rows | rows, 1.4 ms |
| `?currency=DEM` (present, rare) | 6.7 s | 6.2 s — unchanged, and correctly so |

The DEM case is the boundary of what this issue can fix: a reachability probe proves that NOTHING
matches, never that a match is near, so a present-but-rare value still walks. That is issue 273's
deferred step 2, cross-referenced rather than absorbed.

The `/docs` performance footnote now states both halves — every isolation-routed filter has a
probe, and a present-but-rare value still walks — with the DEM timing named, so the page can be
checked against the service rather than believed.
