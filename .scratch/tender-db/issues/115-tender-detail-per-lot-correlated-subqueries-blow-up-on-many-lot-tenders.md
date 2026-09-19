# 115 — tender_detail's per-lot correlated subqueries blow up on many-lot Tenders

Status: RESOLVED-VERIFIED 2026-09-19 (board sweep) — `GET /v1/tenders/7161565` (2,604 lots, the issue's own probe) serves in **0.62–0.69 s** on two consecutive hits against **248.8 s** on 2026-08-03: the regime the tail of this record says a single clock can tell apart. The scaling-ratio protocol was not run; a 380× change on the named probe is not a quiet-box artefact. Was: landed on main (merge `3485e3d`, 2026-08-08) — prod verification pending the deploy. Original note: fixed on branch `issue115-set-based-lot-summary` (`2751ce3`, off the deployed `1830d50`) —
awaiting an on-box timing before it ships. Pre-existing perf defect, surfaced 2026-08-03 during the
lots-scan latency fix (diagnosed while deploying `1830d50`). NOT caused by that fix; older.
Kind: performance
Blocked by: —
Blocks: the bounded-seek/truncation fix (issue 116) — must land first, so the honest answer is cheap.
(NOT because returning all lots "makes this worse": the cap bounds the answer, not the work — measured.)

## Verify

    curl -s -o /dev/null -w '%{time_total}s\n' --max-time 120 https://tenders.zebreus.click/v1/tenders/7161565

- **done**: under two seconds — the set-based lot summary serves the 2,604-lot tender (read 2026-09-19: `0.686611s`, `0.616098s`)
- **open**: tens to hundreds of seconds — the per-lot correlated subqueries are back (2026-08-03: `248.8s`)

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

## Direction (designed 2026-08-03) — SUPERSEDED, see "Resolution" at the bottom

> Kept for the record. This plan was **right but incomplete**: it removed one of the two quadratic
> terms and left the read still 15.4× superlinear. Do not implement from this section — the shipped
> fix also changes the driving table. Its "no planner dependence" and "one code path" claims did not
> survive measurement.

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
2. ~~The sorter is itself a candidate fix, and possibly the cheaper one.~~
   **Considered and REJECTED as the fix — do not revive it as a shortcut.** An
   index-satisfiable ordering would let `LIMIT` truncate before the subqueries run,
   which reduces the number of ROWS running subqueries (all → one page). But each
   surviving row still re-walks the O(lots) satellite slice, because `lot_id` is
   unindexed — so it is **O(page × lots)**, a mitigation of the re-walk rather than
   its removal. It also does nothing for the all-lots case that issue 116 wants,
   where it collapses to **O(lots²)**. The batching fix is **O(lots)**: walk each
   satellite slice once.

   The sorter finding stays recorded above as a **latent fact**, not a pending task:
   the batching fix resolves it too, since sorting bare rows is cheap once the
   per-row subqueries are gone. (Raised by sdk-vendor as a possible cheaper path,
   settled against by team-lead's scaling analysis and proj-fix's scaling-ratio test.
   Recorded because "why didn't we just fix the ORDER BY?" is the obvious question
   for the next reader, and it deserves an answer rather than a rediscovery.)

## Resolution (proj-fix, `2751ce3` on `issue115-set-based-lot-summary`, off the deployed `1830d50`)

The design above was right about the satellites and **incomplete**: batching them out took 2,400 lots
from 30.62s to 0.97s and the ratio stayed at **15.4× — still quadratic**. Ablating the remaining query
term by term found a second, independent quadratic:

| variant (2,400 lots)      | time    | ratio |
|---------------------------|---------|-------|
| full query                | 0.9613s | 15.9× |
| minus the MAX(seq) subquery | 0.9672s | 15.7× |
| **minus the `tender_version_lots` join** | **0.0037s** | **4.1×** |
| bare `lots` table         | 0.0019s | 4.2×  |

**The `tender_version_lots` join was the second quadratic term** — and it is the same turso behaviour
as the satellites, not a different bug: seek a `(tender_id, seq)` index prefix, then walk the whole
slice landed in instead of using the remaining key columns. Here that is egregious, because the table's
PRIMARY KEY `(tender_id, seq, lot_id)` covers the join predicate *exactly*.

**An index does not rescue it.** Creating an explicit index over precisely those three columns and
re-measuring gives 0.9555s → 0.9438s — unchanged. turso will not use any index for that join while
driving from `lots`. So the "just add an index" answer is dead, and the deferred-builder obligation
(issue 111) it would have carried is avoided. That probe is kept as a test
(`lot_summary_cost::an_index_does_not_rescue_the_lots_driven_join`) with an assertion that FAILS if a
future turso learns to use the third key column — at which point this design should be revisited.

### What shipped

Drive the containment question from the containment table. `lots()` now takes one of two shapes:

- **containment** (`filter.tender` set, `Scope::Page`) — drives from `tender_version_lots`, seeking its
  PK prefix on `vl.tender_id = ?`. The `MAX(seq)` subquery becomes **uncorrelated** here, so the
  seventh correlated subquery the plan gate recorded runs once instead of per lot.
- **stream** (everything else) — drives from `lots` in id order, **SQL byte-identical to the deployed
  `1830d50`**. The working path is untouched.

Plus `summarise`: each satellite read once per version, per-lot pick in memory, applied *after* `LIMIT`.
That also disposes of the sorter finding below — the sorter now carries identity-only rows.

The split is not a special case for big Tenders. It is the containment-vs-stream mismatch `1830d50`
named, followed the rest of the way: the set a Tender-scoped read asks for IS the rows of
`tender_version_lots` under one `(tender_id, seq)`.

### Measured

```
BEFORE   600 lots 1.9057s   2400 lots 30.6244s   -> 16.1x for 4x the lots
AFTER    600 lots 0.0037s   2400 lots  0.0169s   ->  4.6x for 4x the lots
```

**1,812× at 2,400 lots, and linear.** Still to do: time it on the box against the real 2,604-lot
Tender (the 16.05s fixture), since every number here is local.

### The candidate fix that was rejected

The sorter finding below suggested an ordering the index can satisfy, so `LIMIT` truncates before the
subqueries run. That is **not a fix for this issue**: it leaves the per-lot shape intact, so the cost
becomes `page × slice` — at the 1,000 rows the endpoint serves, ~6s rather than 16s. And
`/v1/tenders/{id}` asks for the whole set, so the page IS the set and it buys nothing there. It also
treats the truncation as load-bearing, which the same sweep disproved. Worth keeping as a separate
note if the global list ever needs it; it does not resolve 115.

### Consequences elsewhere

- **Issue 112's on-box plan gate will change colour by design.** B1's certified access path was
  `SEARCH l USING INDEX sqlite_autoindex_lots_1 (tender_id=?)`; the tender-scoped statement now drives
  from `tender_version_lots`. The gate needs re-baselining against the new plan — and note that the
  gate passing this read at 248.8s is exactly why its expected plan must not be treated as a
  correctness statement.
- **Issue 116 is unblocked.** Serving all 2,604 lots now costs one index-prefix seek plus three
  satellite reads.

## The sorter survives the fix — and stops mattering (proj-fix, measured)

run-driver asked the right question: if the skeleton still says `ORDER BY l.id`, does the top-level
`USE SORTER FOR ORDER BY` survive, leaving a `LIMIT`ed page still touching every lot? **It survives.**
The post-fix plan of the containment statement, dumped from the builder (not retyped):

```
SCALAR SUBQUERY 1
SEARCH x USING INDEX sqlite_autoindex_tender_versions_1 (tender_id=?)
SEARCH vl USING INDEX sqlite_autoindex_tender_version_lots_1 (tender_id=?)
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
SEARCH t USING INTEGER PRIMARY KEY (rowid=?)
SEARCH v USING INDEX sqlite_autoindex_tender_versions_2 (tender_id=?)
USE SORTER FOR ORDER BY
```

So `LIMIT` still does not truncate before the work — and the decoration fetches the version's whole
satellite slice regardless of page size, a second independent reason. **This is not a second win; do
not report it as one.**

It stopped mattering. Post-fix sweep on a fixture rebuilt to prod's measured slice profile
(FR-only texts ~2/lot, 299 amounts all `lot_id IS NULL`, 2 dates total):

| LIMIT | rows | before (box) | after |
|-------|------|--------------|-------|
| 125   | 125  | 16.25s | 0.0146s |
| 250   | 250  | 16.19s | 0.0147s |
| 500   | 500  | 16.37s | 0.0153s |
| 1000  | 1000 | 16.05s | 0.0152s |
| 3000  | 2604 | —      | 0.0158s |

Still flat — **flat at ~15ms rather than ~16s**. Flat was never the defect; flat-and-expensive was.
The sorter now orders identity-only rows.

Curve on that profile: 651 → 0.0041s, 1302 → 0.0078s, 2604 → 0.0153s. **2× lots → 2.0× time**, against
the pre-fix 2× → ~4×. Quadratic → linear on prod's own profile, ~1,050× at 2,604 lots.

Two readings of the plan text worth keeping, because both mislead:

- `SCALAR SUBQUERY 1`, **not** `CORRELATED SCALAR SUBQUERY` — the 7th subquery (the `MAX(seq)` one) is
  hoisted out of the per-lot loop and evaluated once. Confirmed by the plan rather than assumed.
- `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)` appears in the plan **before and after**, meaning
  opposite things: the 13.2M-row walk when `l` is the outer loop, a one-row lookup of
  `l.id = vl.lot_id` when it is joined from `vl`. **Identical string, opposite meanings — only the
  join order separates them.** So `!contains("SCAN")` accepts the broken shape and
  `!contains("INTEGER PRIMARY KEY")` rejects the correct one. The plan tests therefore compare the
  position of the first `vl` line against the first `l` line: which table is the OUTER loop is the
  only thing that separates linear from quadratic. Anyone re-baselining issue 112's B1 needs this.

## The plan test guarding this read had already drifted (fixed)

`store::lib.rs`'s `a_tender_scoped_lots_read_seeks_the_index_instead_of_walking_rowids` planned a
**hand-written** `(l.tender_id, l.id) > (?, ?)` string rather than the builder's output. This fix
removed that cursor form from the builder — so the test was certifying SQL no code emitted, and
passing. Same artifact-versus-proxy failure as issues 110 and 102 (sdk-vendor flagged the pattern for
`hot_read_plans.sh`'s B1; it was already true here).

Fixed by adding a `#[cfg(test)]` seam, `read::lots_statement`, that returns what the builder builds
without running it, so the test plans the artifact. Paired with a **negative control** that plans the
pre-115 shape through the same discriminator and requires the opposite verdict — one measure, two
statements — so a green is attributable to the fix rather than to the statement being reworded. Both
mutation-tested: disabling the containment shape in the builder fails the real-statement test and
leaves the control green.

## Final artifacts

- **`2751ce3`** — the fix. Reviewed by team-lead, measured by run-driver.
- **`2ea1b23`** — the plan-test fix (artifact-not-copy, negative control, issue-103 orphan,
  prod-profile probe). **Emitted SQL proven byte-identical to `2751ce3`**, by observing
  `Query::rows` (the choke point every read passes through) at both shas and diffing the statement
  and its bound parameters — sdk-vendor's method, so the builder is observed rather than modified.
  The review and the curve therefore transfer as a measured fact rather than an inference.

Either sha carries the same release binary; `2ea1b23` is preferred because it carries the working
plan test.

A note on what was NOT verified: an attempt to confirm the `#[cfg(test)]` seam's absence with `nm`
over the release rlib returned zero hits for both the seam and the production function it wraps —
inconclusive, and not offered as evidence. The seam's absence rests on `#[cfg(test)]` being a
compile-time gate; the SQL diff above is the load-bearing check.

## A cross-tender data leak in the first fix — caught before deploy, by someone else's fixture

`2751ce3` and `2ea1b23` were reviewed, measured and queued to deploy. Both were **wrong**.

`summarise` indexed the page's rows by **`lot_id` alone**, then read each distinct `(tender_id, seq)`
satellite slice and matched the returned rows back by that key. The pre-fix subqueries matched on
**`s.tender_id = t.id AND s.seq = v.seq AND s.lot_id = l.id`** — all three columns. So whenever a
satellite row of one Tender's current version carried a `lot_id` belonging to a DIFFERENT Tender, and
both lots appeared on the same page, the fix decorated the wrong Tender's lot:

```
whole list: tender 20 lot LOT-2 disagrees with the pre-fix SQL
 left:  title: None,                 value_cents: None,       currency: None,  deadline: None
 right: title: Some("cross-tender"), value_cents: Some(4242), currency: Some("PLN"), deadline: Some(…)
```

The old per-lot subqueries could not produce this: their `s.tender_id = t.id` never matched. So the
claim "semantics preserved exactly" was **false**, on the public `/v1/lots` list.

This is the issue-103 orphan shape again, in the **satellites** rather than in `tender_version_lots` —
the sibling of the orphan case team-lead asked for, which neither of us thought to cover.

**Fix:** key the index on the whole of what the subquery matched on.

```rust
let at: HashMap<(i64, i64, i64), usize> =
    rows.iter().enumerate().map(|(i, r)| ((r.tender_id, r.seq, r.id), i)).collect();
```

No extra queries, no change to the driving table or the emitted SQL; the curve is unchanged
(prod-profile 651/1302/2604 → 0.0042 / 0.0080 / 0.0150s, against 0.0041 / 0.0078 / 0.0153s before).

### The rule this yields

**A test passing, and being mutation-tested, does not prove its FIXTURE exercises the new failure mode
the change introduces.** Mutation-testing proves the test can fail *on the axis it varies* — it says
nothing about whether that is the right axis. The original fixture varied tie-breaks within one Tender
and was provably sensitive to all of them; the failure mode lived on an axis it held constant.

So mutation-testing answers "can this test fail?" and leaves "can this test fail *for the reason this
change could break*?" unanswered. Only the second question is worth a gate.

### Why the original equivalence test could not see it

The oracle was sound. The **fixture** was too narrow: one Tender, therefore one version, therefore no
way for a version's slice to reach another version's lot — the precise failure mode the rewrite
introduced. It was mutation-tested and strong *on the axis its author thought to vary* (tie-breaks
within a Tender) and blind on the axis he did not.

What caught it was an independently written fixture — a page spanning three Tenders at different
current seqs, with repeated lot keys and a deliberate cross-tender satellite row — added to the same
test file by **proj-fix-2**, the agent spawned to cover this role during an API outage and stood down
after. Someone who had not written the code, and who therefore did not inherit the assumption. The lesson is not "write more cases"; it is that **the author of a
rewrite is the worst person to choose its fixture**, because the same blind spot shapes both. Where a
change introduces a new failure mode, the fixture should come from someone who did not write the
change.

### Blast radius (open question for the box)

Requires a satellite row whose `(tender_id, seq)` is a Tender's current version and whose `lot_id`
belongs to another Tender. Issue 103 records that lot rows do get orphaned, so this is not assumed
hypothetical. Asked run-driver to count them on the snapshot:

```sql
SELECT COUNT(*) FROM tender_version_texts s
  JOIN lots l ON l.id = s.lot_id
 WHERE s.lot_id IS NOT NULL AND l.tender_id <> s.tender_id;
```

Zero means the defect was latent. Non-zero means `/v1/lots` would have served cross-tender values from
the moment `2751ce3` shipped, and it belongs on issue 103 as well.

### It shipped, and the blast radius came back zero

Messages crossed: `2751ce3` was deployed at 10:33:55 UTC before the stop-request landed. So this was
not a near-miss on an unshipped artifact — the defective build served traffic.

run-driver answered the rollback question properly, and with a better instrument than the one asked
for. Rather than counting orphans in the schema, they asked what actually decides a rollback — *does
the live endpoint return what the pre-fix semantics would have returned?* — recomputing every served
lot's title/value/deadline from the DB via the old three-column match and comparing:

| sample | lots | distinct Tenders per page | title | value | deadline |
|---|---|---|---|---|---|
| head of stream, 6 × 200 | 1,200 | 56–117 | 0 | 0 | 0 |
| deep cursors (500k/2M/5M/9M/12M), 5 × 100 | 500 | 10–71 | 0 | 0 | — |

Every page spanned dozens of Tenders — the exact trigger condition — with **zero divergence**. The
defect was latent in the data as it stands.

Stated precisely, because the number is easy to over-read: 1,700 of 13.2M lots is a sample, and
issue-103 orphans would plausibly be *clustered* (one bad rewrite touching one Tender's slice) rather
than uniform. This is strong evidence of **no widespread contamination**, not proof of none.

Decision: hold rather than roll back, then roll forward to `a39d53a`. Reverting to `1830d50` would
have traded a defect with zero observed occurrences for a 248.8s endpoint that is a live availability
problem.

### A second instrument failure, caught by its own author

run-driver's FIRST comparison harness reported 119 value and 119 deadline mismatches. All false: bash
`read` with `IFS=$'\t'` collapses consecutive tabs, so one empty field shifted every later column —
the tell was a "cents" field containing `2026-06-30T10:00:00+02:00` — compounded by `jq @tsv` escaping
newlines that the stored values carry literally. Rewritten in Python against `sqlite3` with
parameterised queries and typed comparison, it returns the zeros above.

**Had that first run been reported it would have triggered an emergency rollback on a harness bug.**

The lesson generalises past tabs, and is the same one as the fixture above: the instrument was wrong
and its output was *plausible*. Worth stating in the form that is easy to get backwards — a
**disagreement** result deserves exactly as much scepticism as an agreement result. The temptation is
to audit a green and trust a red, because a red confirms the fear that motivated the check.

### The pattern behind all three near-misses: state what an instrument cannot see

Three instrument failures happened within a few hours of each other, none of them careless, all of the
same shape — **a sound instrument aimed at something narrower than the claim it was used to support**:

1. **The one-Tender equivalence fixture.** Oracle-backed, mutation-tested, and structurally incapable
   of reaching the multi-version failure mode the rewrite introduced. One Tender means one version.
2. **The post-deploy correctness check.** run-driver certified the deploy green on `/v1/tenders/{id}`
   — single-Tender by construction, so it could not have caught a cross-Tender leak. Their words:
   "proving the tender-scoped path correct says nothing about the stream path."
3. **The tab-collapsing comparison harness.** Reported 119 mismatches that did not exist; would have
   triggered an emergency rollback on a `bash` `IFS` artifact.

None of these were caught by being more careful. (1) was caught by an independently written fixture,
(2) by a stop-request that said where to look, (3) by its own author re-implementing it. The catch
chain for the leak ran through all three people; no one of them would have reached it alone.

**A fourth variant, distinct from the other three: asserting about SOMEONE ELSE'S instrument from
outside it.** proj-fix warned sdk-vendor that their section-E `git diff --quiet <rev> <tip> -- read.rs`
stamp check would fail, and named the four commits responsible. It passed. The drift was measured from
`a39d53a` — the *deployed* rev, the natural reference for the person measuring — while the check reads
from the *stamped* rev `3c5ae52`, which had already absorbed all four. Acting on the warning would have
meant regenerating a checked-set against a stale premise, on a check that was already green.

The other three are all someone being wrong about an instrument they built. This one is being wrong
about an instrument someone else built, by reasoning from one's own reference point and asserting it
about their mechanism without checking what that mechanism keys on. It is worse in one respect: the
recipient had no reason to doubt it, and only caught it by verifying rather than trusting. **A warning
intended to save someone time will cost it unless the sender checks the receiver's frame of reference,
not their own.**

The cheap practice that would have caught all three: **report what an instrument cannot see alongside
what it reports.**

### The question that operationalises it

Naming the failure mode did not prevent committing it — four times in one day, each after recording the
pattern and citing it to someone else. Knowledge was not the defence. What worked, every time, was
running one specific question before reporting:

> **What would a correct-but-slow version of this look like, and can my check tell it from the fixed
> one?**

If the check cannot distinguish them, it is on the wrong axis and cannot fail for the reason it claims
to test.

It disposes of each failure in a line. A correct-but-slow lots read returns the same rows — so a
row-count gate is blind to it. A correct-but-slow organizations read is *index-served* — so an
index-name assertion is blind to it. A correct-but-slow short-circuit returns the same page — so an
end-to-end result assertion is blind to it.

The distinction underneath is asking what the CHANGE did rather than what the CHECK is about; the
question is just how to run that. It assists the hard step — naming the property that changed — which
no classification of instruments can do, because every such classification only applies *after* the
naming is already right.

And that is the recursive part, which is the real finding rather than an irony: **every abstraction
produced here was itself an instrument, and each acquired the failure mode it was written to describe.**
The rule about paraphrases was violated by its author two hours after writing it; a guard against
silently-skipped tests had a silent coverage bug; a taxonomy of instrument-blindness is blind in exactly
the way it classifies. The only defence any of it produced is the habit of running the specific check
before reporting. "Green on `/v1/tenders/{id}` — does not exercise the multi-Tender path" costs one
clause and converts a false all-clear into a scoped one. Likewise a fixture's doc comment should say
which axes it does NOT vary, since that is where the next defect will live.

Corollary, in the direction that is easy to get backwards: **a disagreement result deserves as much
scepticism as an agreement result.** The instinct is to audit a green and act on a red, because a red
confirms the fear that motivated the check — which is precisely how the 119 phantom mismatches would
have caused a rollback.


## Which instrument can verify THIS fix — and which two cannot

115 is a **cost** defect: the rows were always right and the access paths were always
index-served. That places it on the one axis neither of the project's usual instruments
can see, and both blindnesses have now been demonstrated on this issue's own evidence.

| property of a read | instrument that sees it | blind to |
|---|---|---|
| **ANSWER** — which rows come back | row-count gates, result assertions | the path, and the cost |
| **PATH** — how they are reached | 112's plan gate | the cost |
| **COST** — how much work it took | a clock, an execution count | — |

* **A result assertion cannot verify this fix.** The batching returns identical rows by
  design; a test asserting on the answer passes whether or not the fix is present. This
  is not hypothetical — proj-fix wrote exactly that test for the 117 short-circuit
  (`fe2c16b`), found it would have passed with the guard's boundary *anywhere at all*,
  and replaced it with an assertion on `prefix_ranges` itself.
* **A plan assertion cannot verify it either.** 112's gate passed this read at **248.8s**
  with every line index-served (rule 6).

The two are duals — *plan-blind-to-cost* and *result-blind-to-path* — and between them
they account for every false green this project has chased, including the row-counting
gates that stayed green through the entire 2.2s `lots_of` outage. Recorded in the gate's
rule header (`e28928a`).

**So the acceptance criterion is a clock, and specifically a scaling RATIO** across a
lot-count spread with a control that must NOT improve — not a single before/after number,
which cannot distinguish "the batching worked" from "the box was quiet". Stated here as
well as in 112 because this is the issue whose sign-off decision it governs.