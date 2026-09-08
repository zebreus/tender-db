# 366 — the head columns elect MAX over facts that carry no quality flag: a 2005 tender is served as open, and €49 quadrillion tops the value ordering

Status: ready-for-agent — **UNIT 1 DECIDED 2026-09-08 (owner), see "Unit 1 DECIDED": two flag legs (negative + all-9s sentinels, 15,899 rows; >€100bn implausible, 175 rows), with the €10–100bn band explicitly left to the lot-sum/FMTVAL signals because no threshold separates the NHS England contract from a €10bn vending-machine notice.** Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings;
unit 1 is the decision 132 has been holding)
Kind: defect (canonical head derivation + read filters) — data-profile rules 8/9/12 need
somewhere to live before they can be enforced
Relates to: 132 (rule 8 named as "the one remaining decision … a canonical-semantics
change, to be decided deliberately"), 171 §3 (rules 8/9/10/12/14 — seeded, never wired),
267 (the plausibility MEASURE, explicitly not a gate), 216 (the head columns), 343 (the
fold-vs-read tie-break; here both sides agree on a rule nobody justified), ADR-0014 D5

## Observed

**Magnitudes.** `SELECT id, current_value_eur_cents, current_title FROM tenders ORDER BY current_value_eur_cents DESC LIMIT 6` → 4490098 = 4,970,000,000,000,000,000 EUR cents ("Study on anti-corruption measures in EU border control") = **€4.97 × 10¹⁶**; 4576720 = 1,159,393,740,000,000,000; 4489350 = 598,403,000,000,000,000; 4490942 = 400,000,000,000,000,000. Counts: **174** tenders over €100bn, **387** over €10bn, 89 over 10¹² EUR cents. `/v1/tenders/43065` serves `value {cents: 25756286172000000, currency: PLN}` = 257.56 trillion PLN while the same row's `lot_results` award 181,515,452.29 PLN; `?min_value=1000000000000` returns it on page 1.

The parse layer shows the shape: notice 12376354 carries a PROCEDURE-scope `TED-VALUE_COST` of 4,970,000,000,000,000,000 cents beside a LotResult `TED-VALUE_COST` of 497,000,000 cents (€4.97M) and two `TED-VALUE` of 4,970,000 — exactly 10¹⁰ apart. 4576720's 1.15939374e18 is likewise exactly 10¹⁰ × €1,159,393.74.

**Sentinels pass the bounds.** `?max_value=100&limit=5` → id 26 `{cents:1}`, id 34 `{cents:-100}` ("Beschaffung von 3 Nutzfahrzeugen" — the -1.00 publisher sentinel, served as the row's value with no marker). `?max_value=0&limit=3` → 34, 49, 98.

**A closed 2005 tender is served as open.** `SELECT tender_id, seq, lot_id, field, utc_seconds FROM tender_version_dates WHERE tender_id=3323836` → exactly two rows, **both at seq 1**, both `lot_id` NULL, both `submission_deadline`: 1,118,793,600 (2005-06-15) and 32,677,554,600 (3005-07-06). One version, notice 2556682, published 2005-05-19 — one notice publishing two deadlines, not a cross-version conflict. `/v1/tenders/3323836` lists both in `dates` and serves `submission_deadline = "3005-07-06T10:30:00+00:00"`. And **`/v1/tenders?status=open&sort=deadline&order=desc` returns 3323836 FIRST**.

**The deadline extremes own both ends of the ordering.** `tenders_current_deadline` ASC: 5671586 = 0016-06-09, 1542367 = 0025-02-01, 5653434 = 0206-06-03, 1466977 = 0 (1970-01-01). DESC: 3323836 = 3005-07-06, 1154724/838350 = 2999-12-31, 279672/267598 = 2924, 3380840 = 2205-11-18. 35 in 2099.

## Why, exactly

**One sentence:** the head columns are an unconditional extremum over the head version's facts, and the canonical fact types have nowhere to record that a fact is implausible — so the worst value in a version is elected as the tender's headline and then owns the ordering, the bounds and the status.

- `head_value_eur_cents` is `.max()` over every tender- and lot-level `Fact::Amount` of the head version — `crates/store/src/canonical.rs:1239-1249`, the `.max()` at **:1248** — written to `tenders.current_value_eur_cents` (canonical.rs:8616). `head_deadline` is `.max()` over every `submission_deadline` `Fact::Date` — `crates/store/src/canonical.rs:1176-1187`, the `.max()` at **:1186** — written to `tenders.current_deadline` (canonical.rs:8614).
- **There is nowhere to put a flag.** `Fact::Amount { field, cents, currency, tax_basis }` and `Fact::Date { field, utc_seconds, offset_minutes, has_time }` — `crates/store/src/canonical.rs:1147-1148` — carry no quality field, and the satellites mirror them. That is exactly why data-profile rule 8 ("store as published, flag `sentinel`, exclude from canonical value columns") is still an undecided seed on 132 and 171, and rule 12's placeholder null-out likewise.
- **The only magnitude bound in the pipeline is i64 overflow.** `cents()` fails only when `checked_mul(100)` overflows (`crates/ingest/src/eforms/value.rs:114-119`), and that failure is what raises `unrepresentable-value` (`crates/ingest/src/eforms/parse.rs:258`) — so the "10⁵⁰ class" is quarantined by a *representation* limit, not a plausibility rule, and everything below ~9.2e16 EUR is accepted verbatim. The legacy era is weaker still: an unconvertible r209 amount is not quarantined at all, the raw text is kept (`crates/ingest/src/r209/parse.rs:437-443`).
- **The plausibility work built a measure, not a gate.** `amount_plausibility_template` COUNTs `a.cents > 100000000000000` per profile in the weekly report — `crates/ingest/src/data_quality.rs:689-698`, the constant at **:694** — and its own doc says it "is not a correctness claim about any single amount". Nothing consumes the count, and that line is the only magnitude constant in the non-test codebase.
- **The read side repeats the election instead of correcting it.** The list/detail deadline pick is the same rule in SQL — `ORDER BY s.utc_seconds DESC LIMIT 1`, `crates/store/src/read.rs:1435-1443` — and `status` is a range predicate on `current_deadline` justified by "MAX(d) > now ⟺ EXISTS(d > now)" (`crates/store/src/read.rs:768-778`), which is precisely why one implausible sibling makes a closed tender open. The value bounds compare the head column with no floor and no sentinel exclusion (`crates/store/src/read.rs:855-871`).
- **Two co-published deadlines are structurally normal.** Several source ids map to the one canonical `submission_deadline` — `TED-DATE_RECEIPT_TENDERS`, `TED-RECEIPT_LIMIT_DATE`, `TED-NEW_VALUE.DATE`, `TXT-DT`, `TXT-DD` at `crates/ingest/src/project.rs:215-241` — and `Fact::key` groups them all under `("date","submission_deadline")` (`crates/store/src/canonical.rs:1157-1165`), so `supersede` cannot drop either: they coexist in one version's BTreeSet and MAX arbitrates. This is issue 343's shape one field over, except that there the two sides disagreed; here they agree on "latest wins", a rule chosen for the status filter and never justified as a display rule.
- **One upstream contributor, uninspected**: `Rule::Amount` prefers `@FMTVAL` over the element text unconditionally and never compares the two — `crates/ingest/src/r209/parse.rs:426-428` — so a wrong eSender machine attribute is adopted and the human-readable figure discarded. The exact 10¹⁰ factor on two independent 2011 EU-institution notices fits that shape.

## Units

1. **Decide rules 8/9/12** (owner's call, first unit — this is 132's open decision): a quality flag on `Fact::Amount` / `Fact::Date` and the amount/date satellites, plus a per-currency ceiling and the sentinel/placeholder lists. Record the reasoning here.
2. **Head election skips flagged facts**: `head_value_eur_cents` (canonical.rs:1248) and `head_deadline` (canonical.rs:1186) fall back to the best unflagged fact, or NULL. Fixtures: 43065, 4490098, 3323836.
3. **State the deadline tie-break once** — a plausibility leg, then a stated rule rather than "latest" — and make canonical.rs:1186 and read.rs:1440 read the same ladder, the 343 way. 3323836 must serve 2005-06-15 and must not appear in `status=open`.
4. **Value bounds** exclude flagged amounts (`crates/store/src/read.rs:855-871`), so `max_value=100` stops returning the -1.00 sentinel and the zeros.
5. **Compare `@FMTVAL` against the element text** in `Rule::Amount` (`crates/ingest/src/r209/parse.rs:426`) and flag or prefer on disagreement. Needs one archive-member read first — gated per `docs/agents/prod-box-reads.md`.
6. **Make 267's measure a signal**: the weekly report lists the top N by magnitude per currency so the class is visible rather than counted.

## Unit 1 DECIDED (owner, 2026-09-08) — and the measurement moved the rule

Unit 1 asked for "a per-currency ceiling and the sentinel/placeholder lists". Measuring the tail first
changed what the rule should be: **the dominant defect is not magnitude, it is a handful of exact
published values**, and a magnitude threshold alone cannot separate the rest.

### What the tail actually looks like

Counts of `tenders.current_value_eur_cents` above each threshold:

| above | tenders |
| --- | --- |
| €1e15 | 8 |
| €1e14 | 12 |
| €1e13 | 25 |
| €1 trillion | 89 |
| €100 bn | 175 |
| €10 bn | 394 |
| €1 bn | 2,902 |

Every one of the top 25 is absurd on its face — €4.97e16 for "Study on anti-corruption measures in EU
border control", €5.98e15 for "Renovation of 4 Linac flight path cabins", €65 trillion for cleaning
services. The 25th row is still €11.7 trillion.

### Exact values, and they are the bulk of it

| value | tenders |
| --- | --- |
| **−1.00** | **15,529** |
| any negative | 15,650 |
| 999,999,999 | 199 |
| 9,999,999,999 | 36 |
| 99,999,999,999 | 14 |
| 1,000,000,000 | 56 |
| 10,000,000,000 | 12 |
| 100,000,000,000 | 2 |
| 0 | 24,512 |

The all-9s family is a **form-width maximum**, and the counts falling with width (199 → 36 → 14) are its
signature. Reading the €10–100 bn band top-down confirms it from the other side: it is dominated by
`99,999,999,999` and `100,000,000,000` sitting on tiny municipal contracts — "Étanchéité terrasse"
(roof waterproofing), "Gazole non routier" (off-road diesel), "Épinal", "Assurance des risques
statutaires". €100 bn for a roof is not a magnitude error, it is the widest number the form accepted.

### The decision

**Leg A — `sentinel`, by exact value.** Flag, exclude from the head columns and from the value bounds,
keep the published row:
1. **any negative amount.** No procurement has a negative value; −1.00 alone is 15,529 rows and is a
   documented publisher convention for "not stated".
2. **an all-9s run of ≥9 digits in the major unit** (999999999, 9999999999, 99999999999, …) — 249 rows,
   evidenced above as a field-width maximum rather than a figure.

**Leg B — `implausible`, by magnitude.** Flag amounts **above €100 bn EUR-equivalent** (1e13 cents).
175 rows, and inspection of the top 25 plus the 99999999999 cluster shows the class is junk throughout.
This is deliberately far above any real single procurement: the whole EU procures on the order of
€2 trillion a year across every member state, so one notice at €100 bn is not a close call.

**Explicitly NOT flagged, and why:**
- **Round powers of ten** (1e9 → 56, 1e10 → 12, 1e11 → 2 rows). A €1 bn framework is a real thing; the
  all-9s neighbours are not. Flagging round numbers would delete genuine values to catch nothing the
  all-9s rule misses.
- **Zero** (24,512 rows). Ambiguous by construction — a planning or market-engagement notice
  legitimately publishes 0 (UK FTS release 083645 does exactly this, issue 342). It stays, and
  `?max_value=0` returning zeros is correct; what that filter must stop returning is the negatives.
- **The €10 bn–€100 bn band** (219 rows between the two thresholds). This is the honest limit of a
  magnitude rule: the band genuinely mixes real mega-frameworks — "NHS England 2019/21 Specialised
  Commissioning Contracts" €10.6 bn, the Île-de-France transport authority contract €10.8 bn — with
  obvious junk like "Homecare and Support on the Isle of Wight" €10.5 bn (population ~140,000) and
  "Vending Machine Services" €10.65 bn. **No threshold separates those**, and picking one would either
  delete the NHS contract or keep the vending machines. Separating them needs the tender-amount-versus-
  lot-sum ratio and unit 5's `@FMTVAL`-versus-element-text comparison — the two signals that can tell a
  10¹⁰ scale error from a big contract. That is the next unit, not a number guessed here.

**Rule 9/12 (placeholder null-out):** subsumed. A flagged fact is not nulled — it is stored as
published and skipped by the election, which is rule 8's disposition and keeps the parse layer
faithful (ADR-0004). Nothing needs a separate null-out path.

**Shape of the flag:** one nullable `quality TEXT` on `Fact::Amount`/`Fact::Date` and their satellites,
carrying `sentinel` or `implausible` — not a boolean, because the two have different causes and the
report should be able to say which. Deadlines get the same column for unit 3's ladder; the year-3005
value is `implausible` by a date ceiling (a deadline more than ~10 years out), decided with unit 3.

## Units 2 + 3 done (`aa732c5`) — and what the golden test proved

`head_value_eur_cents` and `head_deadline` now filter before `.max()`:
`sentinel_amount` (negatives, all-nines runs ≥9 digits), `IMPLAUSIBLE_EUR_CENTS`
(€100 bn), `DEADLINE_HORIZON_SECS` (ten years past the notice's own publication).
Four tests, including the 3323836 pair and the "two plausible deadlines still take
the later one" case that pins the horizon as an exclusion rather than a new tie rule.

**The golden snapshot did not move.** That is the informative result: the fixture
corpus contains no sentinel and no over-ceiling amount, so the change is provably
inert on well-formed data and reaches only the junk class. No regeneration, and no
argument about whether a reviewed derived-layer change was being waved through.

### Not decided: how standing rows pick it up

New and re-folded Tenders get the new election immediately; the ~16,000 already
written do not. Two routes, and the choice is a real one:

- **`PROJECTION_EPOCH` bump** — the mechanism that exists, and it re-folds all
  7,929,584 Tenders to correct roughly 16,000. Hours of box time, and it re-derives
  everything else at the same time, which is both its cost and its only advantage.
- **A targeted repair job** — select the affected rows (negative, all-nines, over
  ceiling, deadline beyond horizon), recompute just their head columns, dry/wet with
  a tolerance abort like `repair-notice-instants`. Cheap and auditable, but it is a
  second implementation of the election that can drift from the fold's.

The drift risk is what makes this worth deciding rather than defaulting: issue 343
and this issue are both "two places computed the same election and disagreed". A
repair job that recomputes head columns is exactly that shape again. Leaning to the
epoch bump for that reason, run in a quiet window — but it should be decided with
the re-fold's cost measured, not asserted.

### Baseline for verifying the re-fold (read off prod 2026-09-08, code deployed but rows not yet re-folded)

| tender | `current_value_eur_cents` | `current_deadline` | expected after re-fold |
| --- | --- | --- | --- |
| 34 | **−100** | 2025-10-02 | value drops (sentinel: negative) |
| 43065 | **6,010,100,611,830,592** (€60 tn) | 2026-02-17 | value drops (over ceiling) |
| 4490098 | **4,970,000,000,000,000,000** (€4.97e16) | 2011-04-08 | value drops (over ceiling) |
| 3323836 | NULL | **3005-07-06** | deadline becomes 2005-06-15, and it leaves `status=open` |
| 26 | 1 | 2023-11-28 | **unchanged** |

Tender 26 is in the table on purpose: €0.01 for a procurement is implausible to a
reader, and the rule deliberately does not catch it. A one-cent amount is not a
form-width maximum and not a negative, and there is no evidence yet about what that
class IS — a placeholder, a unit error, or a real nominal contract. Guessing a floor
would be the same mistake as guessing a ceiling in the €10–100 bn band. If it turns
out to matter it needs its own measurement, and this row is where to start.

### The sentinel list was derived by CONFIRMING guesses, not by discovery — so unit 6 became a detector

Asked directly whether the data had been reviewed for MORE sentinels, the honest answer was no. Every
entry in Leg A and Leg B above came from testing a hypothesis somebody already held — negatives,
all-nines runs, round powers of ten, zero. That method cannot find a shape nobody imagined, and the
list is short enough that assuming it is complete would be an unfounded claim. Two sweeps then
established that the missing instrument was harder to build than expected, and both failures are worth
keeping because each one is a trap that will re-present.

**Sweep 1 — negative, and the instrument was the reason.** A frequency sweep of the head column:

```sql
SELECT current_value_eur_cents AS cents, count(*) AS tenders FROM tenders
 WHERE current_value_eur_cents > 100000000
 GROUP BY current_value_eur_cents HAVING count(*) >= 40 ORDER BY tenders DESC LIMIT 30
```

Top repeats: €2 M ×10,114, €1.2 M ×8,908, €1.5 M ×7,996, €3 M ×6,337, €4 M ×5,929 — genuine budgets
clustering on round numbers, no sentinel among them. **But the column is CONVERTED.** A PLN or HUF
sentinel is multiplied by a rate before it lands there, so it arrives as a non-round EUR figure and
cannot cluster at all. The sweep was structurally incapable of finding what it was looking for, and it
returned a clean-looking result while being so. A negative result from an instrument that cannot
register the signal is not evidence.

**Sweep 2 — the corrected query has no bounded route.** On published amounts instead:

```sql
SELECT currency, cents, count(*) AS rows FROM tender_version_amounts
 WHERE cents > 1000000000 GROUP BY currency, cents HAVING count(*) >= 25 ORDER BY rows DESC LIMIT 35
```

`{"error":{"message":"query exceeded the 10s time limit","status":408}}`. The only index on
`tender_version_amounts` is `(tender_id, seq)` — there is no index on `cents` or `currency`, so any
value-frequency query is a full table scan. **Not retried, per `docs/agents/prod-box-reads.md`:** the
cap bounds the wait, not the work, and turso cannot interrupt a statement, so each retry stacks another
uninterruptible scan. Nor is there a compliant workaround — the snapshot ring was removed in August, so
corpus-scale scans have no ad-hoc on-box path at all.

**Which is what makes this a build rather than a query.** The rule's own scope note says the app reading
its own database in-process is not covered by it — that is the service doing its job. So the sweep
belongs in the weekly data-quality report, and it now is one: two whole-corpus queries,
`sentinel_amounts` and `sentinel_dates`, rendering as report section 10 "Repeated implausible values".
It is also the better answer than a one-off: sources publish new junk continuously, so "are there more
sentinels" is a standing question and a single sweep would only ever have answered it for one day.

**The design, and the two constraints that shaped it:**

- **Ranked by REPETITION inside the implausible tail, not by magnitude.** Sweep 1's real lesson is that
  frequency alone is not a sentinel detector: genuine round budgets dominate the top of a frequency
  ranking at every magnitude, so the discriminator has to be implausibility, with frequency ranking
  *within* it. A value carried by thousands of rows at a magnitude no procurement reaches is a sentinel
  whether or not anyone predicted its shape — which is the discovery property the guess-confirming
  method lacked.
- **The floor exists for hash state, and it is a real limitation.** `GROUP BY currency, cents` holds one
  entry per DISTINCT value; over the full corpus that is millions of them, which is issue 278's state
  blow-up and issue 337's leaked temp database. So amounts are swept at/above 1e11 cents (1 bn major
  units) plus **all** negatives (~15,650 rows, small enough to group unbounded). **A low-magnitude
  sentinel is therefore invisible to this**, and that gap cannot be closed by widening the floor — it
  needs a different discriminator, and it is recorded below as open rather than papered over.
- **Whole-corpus by necessity, not by nature.** The population *is* window-sliceable, but
  `HAVING COUNT(*) >= 10` is not: a value repeating nine times in each of 25 windows passes the corpus
  test and fails every window's. A windowed form would silently under-report exactly the values it
  exists to find — and would do it invisibly, as a shorter listing. A test pins both labels out of
  `windowed_queries`.
- **Dates were never swept at all.** `3005-07-06` reached this issue by being handed over in a bug
  report, not by being found. `sentinel_dates` sweeps both tails — before 1990-01-01 (where an
  epoch-zero default or a century typo lands) and more than ten years past the run (mirroring
  `DEADLINE_HORIZON_SECS`, so the detector looks where the election now refuses).
- **`tenders` beside `rows`.** One Tender revised 90 times and 9,000 Tenders sharing a placeholder both
  produce a large row count and want completely different fixes; `hits` alone cannot tell them apart.

Two traps caught while building it, both silent-failure shaped:

1. **`strftime('%s','now')` returns TEXT and SQLite orders every number below every string**, so
   `utc_seconds > strftime(…)` is always-false and the date sweep would have reported a clean corpus
   forever. The arithmetic (`+ SECS`) forces numeric affinity and is load-bearing rather than cosmetic.
   `fresh_holds_sql` has been correct for the same reason since issue 246, without the reason written
   down. Now pinned by a test.
2. **`as_u64` clamps negatives to zero**, which would have erased the largest sentinel class in the
   corpus (−1.00, 15,529 tenders) on its way through assembly and rendered it as a harmless `0.00`. A
   signed reader (`as_i64`) is the fix; a test asserts the sign survives assembly *and* render.

The listing is deliberately CANDIDATES, not a verdict: it says what has earned a look, and a value's
disposition — a new leg in `sentinel_amount`, a wider horizon, or a note here that the value is genuine
— stays a decision. When the listing is full at 40 it says so, so a truncated tail is never read as the
whole of one; and a failed query renders `UNMEASURED` rather than "none found", because for a detector
those two claims being confused is the worst available failure.

### First real discovery: two date-sentinel shapes nobody had guessed (probe, 2026-09-08)

The amount tail has no bounded route, but the DATE tail does, and it was sitting there unused:
`tenders.current_deadline` is **indexed** (`tenders_current_deadline`) and, unlike
`current_value_eur_cents`, it is **not converted** — it is raw utc seconds. So a frequency sweep of
both deadline tails is an indexed range seek, bounded, and legal to run per `prod-box-reads.md`. Run
before deploying the detector, and it changed the detector.

**Far tail (beyond the ten-year horizon), grouped by DAY:**

| day | tenders | | day | tenders |
| --- | --- | --- | --- | --- |
| **2099-12-31** | **14** | | 2038-12-31 | 3 |
| **2037-12-31** | **12** | | 2038-12-01 | 3 |
| 2040-12-31 | 4 | | 2038-07-01 | 3 |
| 2039-08-31 | 4 | | 2999-12-31 | 2 |
| 2037-01-01 | 4 | | 2050-12-31, 2036-12-31, 2043-05-31, 2040-12-30, 2039-01-01, 2050-04-30 | 2 each |

Two shapes, **neither of them in the guess-derived list**:

1. **Far-year 31 December.** 2099-12-31, 2040-12-31, 2038-12-31, 2050-12-31, 2036-12-31, 2999-12-31,
   and 2040-12-30 beside it. A "no real deadline" convention, exactly parallel to the all-nines
   form-width maximum on the amount side but expressed as a calendar rather than as digits.
2. **A 2037–2038 concentration** — 2037-12-31 ×12, 2037-01-01 ×4, 2038-12-01, 2038-07-01, 2038-12-31,
   2039-01-01. This is the **32-bit epoch ceiling**: 2^31 seconds lands on 2038-01-19, so a system
   capping at its maximum representable date emits late 2037 and 2038. Nobody would have guessed this
   one, and it is the clearest vindication of building a detector instead of extending a list.

**Near tail (before 1990-01-01): a clean negative.** Exactly four rows, every one a singleton —
1970-01-01, 0206-06-03, 0025-02-01, 0016-06-09. These are typos and epoch-zero accidents, not a
convention, and no repeated sentinel lives there. Worth recording as a measured negative so nobody
re-derives it: the pre-1990 tail needs no rule.

### The probe found a defect in the detector, before deploy rather than after

`sentinel_dates_sql` originally grouped by exact `utc_seconds`. **2099-12-31's 14 tenders are spread
across several times of day** (00:00, 10:00, 11:59, 23:59 …) and its largest single SECOND holds only
**4**. With `HAVING COUNT(*) >= 10` on exact instants, the strongest date cluster in the corpus would
have returned **nothing** — a detector silent precisely where it matters, which is the worst failure
mode available to one and indistinguishable from a clean corpus.

Fixed before the gate re-ran, and the fix is now the pinned property:

- **`GROUP BY d.field, date(d.utc_seconds, 'unixepoch')`** — a date sentinel is a DAY a system emits;
  the time of day is whatever the source's formatter appended. `MIN(utc_seconds)` is the group's
  representative instant: every member shares the group's `date(…)`, so the minimum renders to exactly
  that day, correct by construction, and the row shape stays an `i64` like the amount half.
- **`SENTINEL_DATE_MIN_REPEATS = 3`**, separate from the amount threshold of 10. The populations differ
  by three orders of magnitude — negatives alone are ~15,650 amount rows, while the entire far-future
  deadline tail is ~100 tenders. At 10 the date listing would have reported clean while 2040-12-31,
  2038-12-01 and 2038-07-01 sat in it.
- A test asserts the day-grouping is present AND that the exact-instant form is absent, with the
  measured reason in the comment, so the silent form cannot come back as a "simplification".

**Caveat on the numbers above:** they are per-TENDER, off the head column, whereas the detector sweeps
`tender_version_dates` — every version, every date field — so its counts will be higher than these and
the two are not directly comparable. These establish the SHAPES and the threshold; the report's own
first run establishes the corpus-wide magnitudes.

### A better shape for both sweeps, held pending a measurement (2026-09-08)

Shipped, the two sweeps are `whole_corpus_queries` — one full scan of `tender_version_amounts` and one
of `tender_version_dates` per weekly run, because `HAVING COUNT(*) >= n` cannot be evaluated per window.

There is a shape that avoids both scans, and the argument for it is worth writing down before anyone
re-derives it: **drop the `HAVING` and window them after all.** Make the label the value itself
(`currency` + `cents`, or the field + day), return every distinct tail value with its counts, let
`sum_profile_counts` add them across windows, and apply the repeat threshold and the ranking in Rust.
Then it rides the existing affordability machinery instead of adding two scans.

Both count columns sum exactly, for reasons the codebase already relies on:

- `COUNT(*)` per label is additive over disjoint version sets, like every other windowed count.
- `COUNT(DISTINCT tender_id)` is additive **because the distinct key IS the window key** — windows
  partition `tender_id`, so no tender straddles two of them. That is precisely the argument
  [`MERGE_SQL`] carries for its inner `SELECT DISTINCT v.tender_id`, and it is the argument that must
  be made explicitly, because a `DISTINCT` summed across windows is normally wrong.

Hash state stays bounded per window (distinct tail values within one 250,000-id slice), which was the
whole reason for the magnitude floor — so the floor could even be lowered, which is the one thing that
would open the low-magnitude blind spot recorded above.

**Not done, deliberately.** The current form is correct; the redesign is an optimisation, and its
premise — that two extra scans are a cost worth restructuring for — is unmeasured. Job 816 (the first
run of the shipped version) reports per-window timings in the log and runs the whole-corpus phase after
the 32 windows, so the two scans can be bracketed from journalctl timestamps. **Measure that first.**
If the two scans are marginal against a job that already takes ~3 hours for 16 queries × 32 windows,
this is churn on working code; if they are minutes each, do the redesign and fold the magnitude ranking
(unit 6's original wording) into the same single pass rather than adding a third scan for it.

That sequencing is also why the magnitude-ranked listing has not been added yet: adding it as another
`whole_corpus_query` would be a THIRD full scan of the same table, and if the redesign happens both
rankings come free from one windowed pass.

### Section 10's first corpus-wide run (job 816, 2026-09-08) — two new shapes, and a correction to my own record

Job 816: `ok`, 6,396 s, **0 labels unmeasured** — both sweeps executed against the full corpus.

**Two sentinel shapes neither the guess-derived list nor the head-column probe contained:**

1. **`1899-12-31` — 415 rows over 124 tenders**, on the `duration_*` fields. This is the
   **Excel/Lotus epoch**: a spreadsheet whose day 0 is 1899-12-31 (1899-12-30 in the other system)
   exporting a blank or zero date. Nobody had named it, and no magnitude or all-nines rule reaches it.
2. **PLN `22,222,222,222.00` — 250 rows, 1 tender.** A **repdigit that is not nines**.
   `store::canonical::sentinel_amount` tests for a run of ≥9 **nines** in the major unit, so it misses
   every other repeated digit. The fix is to generalise that leg from "all nines" to "one digit
   repeated ≥9 times" — the field-width-maximum argument that justified the nines leg applies
   identically to a key held down.

**Correction to this issue's own record.** The probe recorded above concluded the pre-1990 tail was
"a clean negative — four singletons, no convention, no rule needed". **That is wrong as stated.** It was
measured on `tenders.current_deadline`, which is only the head-elected *submission_deadline*; the sweep
reads `tender_version_dates` — every date field on every version — and there the pre-1990 tail holds the
1899 cluster above. The claim was true for deadlines and false for the corpus. Recorded rather than
quietly edited, because the mistake is instructive: a head-column probe answers a question about the
head column, and generalising it to "the corpus" is exactly the error the converted-EUR sweep made in
the other direction.

**The far-year December pattern is much larger than the head probe showed**, and it pairs 30 with 31:

| field | day | rows | tenders |
| --- | --- | --- | --- |
| participation… | 2039-12-31 | 5,603 | **5** |
| duration… | 2036-12-30 | 1,026 | 365 |
| duration… | 2099-12-31 | 944 | 211 |
| duration… | 2099-12-30 | 748 | 228 |
| duration… | 2037-12-30 | 514 | 194 |
| duration… | **1899-12-31** | 415 | 124 |

`-12-30` recurring about as often as `-12-31`, together with the 1899 cluster, is consistent with
spreadsheet serial-date handling (the two Excel epoch systems differ by one day). **Hypothesis, not a
conclusion** — it wants one archive-member read to confirm, which is unit 5's gated read anyway.

Note `participation… 2039-12-31`: **5,603 rows over 5 tenders**, ~1,120 date rows per tender. The
`tenders` column earning its keep on the first run — by rows alone this is the corpus's biggest date
cluster; by tenders it is five records with enormous version chains, which is a completely different
finding and probably a fold-cost observation (issue 92) rather than a sentinel one.

### Two defects in the instrument, exposed only by running it

1. **The amount floor is currency-blind.** `SENTINEL_AMOUNT_FLOOR` is 1e11 cents = 1,000,000,000 major
   units regardless of currency — which is ~€40 M in CZK and ~€2.5 M in HUF. So the top-40 is dominated
   by ENTIRELY ORDINARY Czech, Hungarian and Swedish contracts (CZK 2,300,000,000 ×1,960 rows/6 tenders,
   SEK 1,200,000,000 ×1,110/63, HUF 1,000,000,000 ×383/110), and the one genuine sentinel in the listing
   (PLN 22,222,222,222) sits at rank ~35. The listing cap is being spent on noise. **Fix: a per-currency
   floor** — the EUR-equivalent ceiling the issue already decided on for Leg B is the right basis, since
   `eur_cents` is on the row.
2. **The scope column is too narrow.** At `{:<10}` every date field truncates to `duration…`, so the
   listing cannot distinguish `duration_start` from `duration_end` — and that distinction decides
   whether a far-future date is normal (an open-ended framework's end) or wrong (a start). A diagnostic
   that hides the discriminating half of its own key is not finished.

Both listings hit `LISTING FULL at 40`, so both tails are longer than what is shown — the marker doing
exactly what it was added for on the very first run.

### Cost: measured, and the windowed redesign is refused

The open question recorded above ("measure that first") is answered. Windows finished at 6,268 s
elapsed; the job totalled 6,396 s. The whole-corpus phase — **all four** queries, including
`fresh_holds` and `longest_chain`, which pre-date this change — cost **~128 s, about 2 % of the run**.
The two new scans are a fraction of that, against a job whose windowed phase costs 6,268 s (`awards`
alone is 2,212 s).

**So the windowed redesign is churn on working code, and is refused on the measurement rather than on
taste.** Its premise was that two extra scans were worth restructuring for; they are not. The argument
stays recorded above in case the cost profile changes (a much larger `tender_version_amounts`, or a
lowered floor), and one part of it is still worth having: it is the only route that would let the
magnitude floor come down far enough to open the low-magnitude blind spot.

### Still open in this issue

Unit 4 (value bounds in `read.rs:855-871` excluding flagged amounts — the read side
still compares against the head column with no floor) and unit 5 (`@FMTVAL` versus
element text, which needs one archive-member read).

**Unit 6 is partly done.** The repetition-ranked detector above is built and is the
half that answers "what shapes exist that nobody has named". Unit 6's original
wording — a per-currency top-N by MAGNITUDE — is a different listing and is still
open: it would show the €4.97e16 rows individually, which repetition ranking never
will because each of them is unique. Cheap to add (a second ordering over the same
bounded tail) and worth doing when someone next reads section 10.

**Two gaps the detector cannot close, stated so they are not mistaken for covered:**

1. **Low-magnitude sentinels.** Below the 1e11-cent floor, frequency ranking is
   dominated by genuine round budgets (measured, sweep 1), and the floor cannot be
   lowered without the hash-state blow-up. Finding a sentinel there needs a
   different discriminator — the tender-amount-versus-lot-sum ratio is the most
   promising, and it is the same signal unit 5 needs for the €10–100 bn band.
2. **The re-fold route for the ~16,000 standing rows** is still undecided (above),
   so section 10's first readings describe a corpus the election has not yet been
   applied to retroactively. Read it as a source census, not as a defect count.

## Done when

- no tender above the per-currency ceiling is served as a head value, and `sort=value`'s first page is re-read and recorded here;
- 3323836 is closed and absent from `status=open`;
- `?max_value=0` returns no negative-sentinel rows;
- `/docs#caveats`' quarantine sentence is corrected (see `served-claims-nothing-re-derives`).

*One issue because:* the €4.97e16 head value, the 257-trillion-PLN row, the -1.00 sentinel in `value`, the year-3005 deadline and the 2005-tender-served-as-open are one derivation — `.max()` over facts that have no field in which to be marked implausible — plus its mirror in the read layer's `ORDER BY … DESC LIMIT 1`.
