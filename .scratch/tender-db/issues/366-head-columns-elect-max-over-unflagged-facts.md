# 366 — the head columns elect MAX over facts that carry no quality flag: a 2005 tender is served as open, and €49 quadrillion tops the value ordering

Status: DONE 2026-09-12 (owner, board re-read) — the four items of 'Still open after this firing' are each closed: the 322-row repdigit drain (2026-09-10, `aa55f6e`), the exact zeros (refused and drained 2026-09-11, `55d239d`), unit 3's unfinished half (payload and head column on one ladder, verified on prod 2026-09-11), and non-EUR sentinels (section 10's sweep, issue 380). Was: ready-for-agent — **STANDING ROWS DRAINED 2026-09-09 (owner), see "The standing-rows route is
ANSWERED": the epoch-bump-versus-repair-job dilemma was a false choice — `refold-notices` aims the
FOLD'S OWN election at the affected notices, so there is no second implementation to drift and no
corpus re-fold. Over-ceiling 175 → 0, negatives 15,644 → 0, beyond-horizon future deadlines 379 → 0;
the five-row verification baseline re-read and matching, `?max_value=0` clean, 3323836 out of
`status=open`. Repdigit field maxima 322 → 0 as well (rev `aa55f6e`), after `7c8a443` added a
nines-at-cent-level leg — re-reading the ordering had found €99,999,999,999.99 standing, which the
rule's `cents % 100 != 0` guard walked past. No sentinel and no over-ceiling value remains in the
head column. UNIT 3 DONE 2026-09-10 (rev `45c18c7`): the display pick — every list shape and the
detail payload — now reads the fold's election instead of repeating it, the deadline by transcribing
the one constant and the amount by looking up the row the fold chose (`s.eur_cents =
t.current_value_eur_cents`), because a digit walk cannot be transcribed without becoming the second
implementation. 3323836 serves 2005-06-15, 4490098 serves €50,000, and the `dates`/`amounts` arrays
still carry every published figure.** The exact zeros are DONE 2026-09-11: decided, shipped
(`55d239d`) and drained (24,039 tenders, 26 rounds, 0 left) — see "The exact zeros, DECIDED" and
"The zero drain" at the end. Next: unit 5's gated archive read for
the €10–100bn band that now tops the ordering, and unit 6's magnitude listing. **Issue 378** is what
the drain's own termination guard turned up: 608 tenders still serve a derived €0, reached by
ROUNDING rather than election. **Issue 375** carries
the third and fourth implementations this turned up — two backfill jobs that would undo the drain.
Earlier: **UNIT 1 DECIDED 2026-09-08 (owner), see "Unit 1
DECIDED": two flag legs (negative + all-9s sentinels, 15,899 rows; >€100bn implausible, 175 rows),
with the €10–100bn band explicitly left to the lot-sum/FMTVAL signals because no threshold separates
the NHS England contract from a €10bn vending-machine notice.** Was: ready-for-agent (filed 2026-09-07 from the external review's verified findings;
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
1. **any negative amount.** −1.00 alone is 15,529 rows. **The reason given here has now failed TWICE
   and the current wording is the third attempt** — see "Correction to Leg A's reasoning" below for
   the first (it is the SDK's withheld marker, not a convention) and **issue 376 for the second: "No
   procurement has a negative value" is simply false.** Waste sold for processing, scrap metal, land
   leases and bank agreements are revenue-side contracts where the supplier pays the authority, and
   the corpus holds them — a NOK 151 M Tromsø bank agreement among them. The DISPOSITION stands, and
   for a better reason: this column means *what the buyer pays*, and a revenue contract is not that.
   Excluding it because the column's domain is expenditure is defensible; excluding it because "no
   procurement has a negative value" is not, and the difference decides whether the fix is a filter
   or a field.
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

### ~~The re-fold route is answered from outside this issue~~ — RETRACTED, see below (2026-09-08)

The undecided route above — `PROJECTION_EPOCH` bump versus a targeted repair job, for the ~16,000
standing rows — is settled by a change on another issue rather than by anything here.

**Issue 369 unit 2c bumps `PROJECTION_EPOCH` anyway.** Its buyer-aware key election changes how Tenders
are GROUPED, so standing tenders cannot pick it up without a re-fold; the epoch bump is not optional
there the way it is here. One corpus re-fold then re-derives both: 369's regrouping and this issue's
head-column election, for the price of the one that was already required.

That dissolves the dilemma rather than deciding it. The targeted repair job's only advantage was
avoiding a full re-fold, and the re-fold is now happening regardless — while its stated cost stands
(a second implementation of the election, which can drift from the fold's, and drift is exactly the
failure issue 343 and this issue both already are). So: **no repair job. This issue's standing rows
ride 369 unit 2c's epoch bump.**

Two consequences to carry:

- **The verification baseline above is the right instrument for both.** The five rows recorded there
  (34, 43065, 4490098, 3323836, 26) must be re-read after that re-fold, and tender 26 must still be
  unchanged — it is the row that pins the rule NOT catching one-cent amounts.
- **Sequencing:** the re-fold must run AFTER the repdigit generalisation (`b100e4a`) is deployed, or the
  ~16,000 rows get re-elected under the narrower nines-only rule and the PLN 22,222,222,222 class stays
  standing until the next epoch bump. `b100e4a` is deployed as of 2026-09-08, so this is satisfied — but
  it is the ordering constraint to check rather than assume if either lands again.

### Section 10's SECOND run (job 817, corrected floor + wide column) — and it re-read the dates finding

Job 817: `ok`, 6,420 s, 0 unmeasured, deployed rev `b100e4a`. Both instrument fixes paid off
immediately, and one of them overturned an interpretation recorded above.

**The wide scope column settled the question it was widened for.** Every far-future date in the
listing is **`duration_end`**; the 1899 cluster is **`duration_start`**:

| field | day | rows | tenders |
| --- | --- | --- | --- |
| `duration_end` | 2036-12-30 | 1,026 | 365 |
| `duration_end` | 2099-12-31 | 944 | 211 |
| `duration_end` | 2099-12-30 | 748 | 228 |
| **`duration_start`** | **1899-12-31** | 415 | 124 |
| `participation_deadline` | 2039-12-31 | 5,603 | 5 |

**That changes the reading, and the earlier framing above was too strong.** A `duration_end` of
2099-12-31 is a publisher saying *open-ended* — an indefinite framework agreement — which is a
CONVENTION carrying real information, not junk to strip. Calling the far-year December cluster a
"no real deadline convention" (recorded above) conflated it with the deadline fields; it is a
contract-END convention. And it never touched the head columns anyway: `head_deadline` elects only
`submission_deadline`, so `DEADLINE_HORIZON_SECS` was correctly scoped the whole time and none of
these rows were ever candidates for it.

What IS defective, now visible because the field is legible:

- **`duration_start` = 1899-12-31, 124 tenders.** A contract cannot start in 1899. The Excel/Lotus
  epoch, confirmed as a genuine defect rather than a convention.
- **`participation_deadline` far in the future** — 2039-12-31 on 5 tenders (5,603 rows), 2060-05-23
  on 1. A participation deadline is a near-term date by definition; these are wrong. Small tender
  counts, so a handful of records rather than a class.

The lesson for the instrument, worth keeping: **section 10 sweeps every date field while the election
filters only `submission_deadline`**, so most of what it lists is not a head-column defect at all.
The "CANDIDATE, not a verdict" line in the render is carrying more weight than it looked like it
would, and the listing must not be read as a defect count.

**The EUR-equivalent floor worked as intended.** The CZK/HUF/SEK ordinary-budget noise is gone, and
what surfaced instead is recorded on **issue 372** — the `-1.00` class is multi-currency (EUR, PLN,
DKK, NOK) and there is a currency literally spelled `unpublished`.

### Correction to Leg A's reasoning: `-1` is a SPECIFICATION, not a convention (see 372)

Leg A above justifies refusing negatives with "−1.00 alone is 15,529 rows and is a documented
publisher convention for 'not stated'". **The disposition is right and the reason is wrong.** The
second run traced it: under BT-195/`FieldsPrivacy` the eForms SDK writes the code `unpublished` and
the number **−1** when a buyer withholds a field — it is the SDK's withheld-value marker, published
alongside the reason and the date it becomes publishable. The committed fixture
`can-withheld-29-00495618-2026.xml` shows it on submission statistics and on an award criterion.

This does not change anything Leg A does — the head election must still skip these, and it does. It
changes what the fix IS: **issue 372 is the root cause** (a withheld field projected as a value at
all, with `notice_withheld_fields` already modelling it correctly one layer down), and Leg A is
defence in depth over it. When 372 lands, this leg stays; its doc comment must stop calling −1 a
convention.

### Unit 4 is SUBSUMED by the re-fold — closed by reading the code, 2026-09-08

Unit 4 asks for the value bounds to "exclude flagged amounts", on the premise that "the read side
still compares against the head column with no floor". The premise is right and the conclusion does
not follow: **because the bounds compare the HEAD COLUMN, units 2+3 already fixed them.**

`version_predicates` is handed `"t.current_value_eur_cents"` by every Tenders shape
(`crates/store/src/read.rs:1589`, `:1857`, `:1906`, `:2445`; the lots shape correlates to the same
column at `:2594`), and `read.rs:1137-1142` says so out loud — "the value bounds compare
`current_value_eur_cents`, a head column". That column is written by `head_value_eur_cents`, which
since `aa732c5` filters `sentinel_amount` and the €100 bn ceiling before `.max()`. So once the rows
are re-folded, `?max_value=0` cannot return the −1.00 rows: they are no longer in the column it reads.

**No read-layer change is needed, and adding one would be the drift this issue is about** — a second
place that decides which amounts count, free to disagree with the fold's. Unit 4 closes as subsumed.

**The non-obvious consequence, worth stating because it is a behaviour change and not a bug:** a
Tender whose ONLY amount was a sentinel now gets **NULL** in the head column (the election filters,
then `.max()` over an empty set yields `None`). SQL's three-valued logic then excludes it from BOTH
`min_value` and `max_value` listings, which is correct — the Tender has no *known* value, so it
belongs in neither bound — but it does mean those tenders leave the value-filtered listings
entirely rather than sorting to one end. Tender 3323836 already shows the NULL shape in the
verification baseline above, so this is representable and served today.

That leaves unit 5 (`@FMTVAL` versus element text, needing one gated archive-member read) as the only
original unit still open, and the magnitude-ranked half of unit 6.

### RETRACTION: 369 unit 2c does NOT bump `PROJECTION_EPOCH`, so this issue's re-fold route is STILL OPEN

The section above concluded that this issue's standing ~16,000 rows could ride issue 369 unit 2c's
epoch bump, so no repair job was needed. **That conclusion is withdrawn. It rested on issue 369's
coupling 2 claiming the bump is required there, and reading the code says it is not.**

`PROJECTION_EPOCH` gates exactly one thing (`crates/store/src/canonical.rs:8748`): whether a tender
whose stored version chain is UNCHANGED may early-return. Its own doc states the condition — a bump is
for when "the same notices now fold to different content", i.e. changed fold LOGIC over an unchanged
grouping. Unit 2c changes the GROUPING, not the content of unchanged groups:

- the three welded tenders lose their key entirely, so their notices form new groups under new tender
  ids, folded fresh against an empty stored chain, and the old tenders are retired by
  `retire_regrouped_nonlegacy_tenders` — none of that consults the epoch;
- the ten-or-so correctly-grouped shaped tenders keep their keys AND their content, so early-returning
  them is right;
- a refused notice that joins an existing legacy component changes THAT component's stored chain, which
  the `keep < stored.len()` comparison already catches — again not the epoch.

So unit 2c needs no bump, and **the dilemma recorded further up is live again**: an epoch bump
(measured in the constant's own doc at **6 h 02 m / 14.2 M version writes for a 2.69 M-notice cohort**,
issue 179 — the whole corpus is larger) versus a targeted repair job that is a second implementation of
the election and can drift from the fold's.

Nothing about the two options changed; only the free ride disappeared. The drift argument still favours
the bump, and its cost is now quantified rather than hand-waved, which is what the earlier note said
the decision needed. **It remains an owner decision to schedule, not a side effect of another issue.**

**Why this is recorded rather than edited away:** the false conclusion was mine, and it was reached by
trusting another issue's prose instead of the code it described — the same mistake as the meta-gate
correction on 369 (which claimed a test would go red when it cannot). Two for two in one session is a
pattern worth naming: **an issue's recorded coupling is a hypothesis about the code, not a reading of
it.**

### Still open in this issue

Unit 5 (`@FMTVAL` versus element text, which needs one archive-member read).
**Unit 4 is closed as subsumed** — see "Unit 4 is SUBSUMED by the re-fold" above: the
bounds already read the head column that units 2+3 fixed, so a read-layer filter would
be a second, driftable decision about which amounts count.

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

## The standing-rows route is ANSWERED — by a third option neither side of the dilemma listed (2026-09-09)

The dilemma recorded twice above — `PROJECTION_EPOCH` bump (6 h 02 m / 14.2 M version writes for a
2.69 M-notice cohort, and the whole corpus is larger) versus a targeted repair job (a second
implementation of the election, free to drift, which is the failure this issue and 343 both ARE) —
was a false choice. **Both options were about recomputing the head columns. A third route recomputes
nothing: hand the affected notices to `refold-notices` and let the fold run its OWN election.**

That has the repair job's cost — only the affected rows — and the epoch bump's correctness, because
there is no second implementation to drift. `refold-notices` unmarks the notices as projected and
stamps their tenders epoch-stale; the incremental fold then re-derives them through
`head_value_eur_cents` and `head_deadline` themselves. Nothing in this issue's code was touched to
make it work; the mechanism already existed for issue 259.

**Why the dilemma looked binary:** both horns were framed as "how do we get the NEW RULE applied to
OLD ROWS", and from there the only question seemed to be whether to re-derive everything or to write
a second derivation. The question that dissolves it is "what already applies the rule?" — the fold —
"and can it be aimed?" It can, per notice.

### What was drained, and the reconciliation

Batched at the `refold-notices` cap of 1,000, one batch at a time, waiting for the queue to go idle
between rounds (`scratchpad/drain366.sh`, `scratchpad/drain366-deadline.sh`).

| leg | selector | before | after | rounds |
| --- | --- | --- | --- | --- |
| over-ceiling amounts | `current_value_eur_cents > 1e13` | **175** | **0** | 1 (jobs 864/865) |
| negative amounts | `current_value_eur_cents < 0` | **15,644** | **0** | 16 (jobs 866–897) |
| beyond-horizon future deadlines | `current_deadline > now AND current_deadline - current_published_at > 315360000` | **379** | **0** | 1 (jobs 898/899) |

Every round moved exactly its batch size (15,644 → 14,644 → … → 644 → 0), which is the check that the
selector and the fold agree: had the fold re-elected a row into the cohort it just left, a round would
have moved less than 1,000.

**The negative count 15,644 against Leg A's recorded 15,650** — the six-row gap is the issue's own
figures being measured a day apart, not a discrepancy in the drain.

### The deadline leg's cohort is narrower than "beyond horizon", deliberately

`DEADLINE_HORIZON_SECS` is relative to the head version's own publication (`d - head.published_at`),
so the full violating set needs `current_published_at`, and a scan. The cohort drained is the
intersection with `current_deadline > now`, which is **the half that causes the visible harm**: those
are the rows `status=open` matches and that own `sort=deadline&order=desc`. A beyond-horizon deadline
already in the PAST is still wrong in the `dates` payload but changes no listing, and it is left to
whatever re-folds those rows next. Sized at 379; not sized for the past half, and that is a gap rather
than a finding.

### The verification baseline, re-read (head columns, off prod)

The five rows recorded above, read straight from `tenders`:

| tender | before | after | expected |
| --- | --- | --- | --- |
| 26 | `1` | **`1`** | unchanged — and it is |
| 34 | `−100` | **NULL** | value drops (negative sentinel) |
| 43065 | `6,010,100,611,830,592` | **NULL** | value drops (over ceiling) |
| 4490098 | `4,970,000,000,000,000,000` | **`5,000,000`** | value drops |
| 3323836 | deadline `3005-07-06` | **`1118793600` = 2005-06-15** | the real date wins |

**4490098 is the informative one.** It did not go NULL — it fell back to €50,000, which is unit 2's
"falls back to the best unflagged fact" doing exactly that rather than the easier "elects nothing".
Tender 26 unchanged at one cent keeps pinning the rule NOT catching that class.

Served behaviour, both "done when" items:

- `?status=open&sort=deadline&order=desc` now tops at **2036-04-30** (3323836 gone). That top is
  itself within horizon for a notice published in 2026, so the ladder is consistent rather than
  merely shorter.
- `?max_value=0` now returns **only zeros** (49, 98, 137, 144, 181, 269). The −1.00 rows are out of
  the column the bound reads, which is the "unit 4 is subsumed" reasoning holding up in production.

### Correction to this issue's own "done when": there is no `sort=value`

"`sort=value`'s first page is re-read and recorded here" names a sort the API does not have —
`/v1/tenders?sort=value` returns *"sort must be 'id', 'published_at' or 'deadline'"*. The ordering
this issue has been talking about throughout is the SQL one its Observed section actually ran
(`ORDER BY current_value_eur_cents DESC`), reachable to a caller only through `min_value`/`max_value`.
Worth fixing in the text because "the ordering is topped by publisher errors" reads as a claim about a
served sort, and it is a claim about a column.

### Re-reading the ordering after the drain found a leg the rule was written to catch and did not

With the over-ceiling rows gone, the top of `ORDER BY current_value_eur_cents DESC` reads:

| cents | major | tenders | title |
| --- | --- | --- | --- |
| 10,000,000,000,000 | €100,000,000,000.00 | 2 | "SPS/CT", "Acquisition de prestations…" |
| **9,999,999,999,999** | **€99,999,999,999.99** | 4 | "Épinal", "Étanchéité"-class municipal work |
| 9,999,999,999,900 | €99,999,999,999.00 | 2 | — |

The first row is the ceiling itself (`IMPLAUSIBLE_EUR_CENTS` is compared with `<=`), which is expected.
**The second row is not.** €99,999,999,999.99 is thirteen nines, and `sentinel_amount` walked past it
because of its `cents % 100 != 0` guard: *"a value with minor units is a figure someone computed, not
a field maximum someone typed."*

The third row, one cent lower, IS caught by the major-unit leg. So the rule was splitting one publisher
behaviour in two on whether the form happened to append two decimal places — and the issue's OWN
evidence for the nines leg cited "Épinal" and "Étanchéité terrasse" at `99,999,999,999`. Same notices,
one decimal shift away.

**Measured completely rather than sampled.** The repdigit-cents values with ≥9 digits are a finite set
(90 of them under `i64`), so each is an index seek on `tenders_current_value_eur` — bounded and exact,
no scan:

| cents | major | tenders | | cents | major | tenders |
| --- | --- | --- | --- | --- | --- | --- |
| 999,999,999 | €9,999,999.99 | **33** | | 111,111,111 | €1,111,111.11 | 12 |
| 9,999,999,999 | €99,999,999.99 | **20** | | 222,222,222 | €2,222,222.22 | 7 |
| 99,999,999,999 | €999,999,999.99 | **14** | | 333,333,333 | €3,333,333.33 | 26 |
| 999,999,999,999 | €9,999,999,999.99 | **2** | | 8,888,888,888 | €88,888,888.88 | 11 |
| 9,999,999,999,999 | €99,999,999,999.99 | **4** | | others (1s–7s) | | 17 |

**Nines: 73 tenders across five widths. Non-nines: 73 across eleven values.**

**The nines are field maxima, and the titles settle it.** All 20 rows at the three highest widths are
small municipal contracts — *Straßenreinigung in der Stadt Gronau* (street cleaning, population
~47,000) and *Aquisição de refeições escolares* (one municipality's school meals) at €999,999,999.99;
*Étanchéité* at €9,999,999,999.99; *Épinal* at €99,999,999,999.99. At €99,999,999.99: road salt, HD
cycloramas, an **Elsevier subscription**, routine building maintenance.

**And the width ladder is the argument, not the titles alone** — it is the same argument the nines leg
already rests on ("the counts falling with width, 199 → 36 → 14, are a form maximum's signature").
The value recurs at nine, ten, eleven, twelve and thirteen nines. A deliberate "must stay under €10 M"
cap — the one genuine reading of €9,999,999.99, and the weakest case here — would appear at ONE width.
Five widths is a key held down.

**Nines only, and the asymmetry is deliberate.** Division genuinely produces a repdigit tail:
€3,333,333.33 (26 tenders) is €10 M / 3 and €1,111,111.11 (12) is €10 M / 9. Those are computed
figures and stay admitted. Nothing divides to a run of nines. The major-unit leg can afford ANY digit
(which is how the sweep's PLN 22,222,222,222.00 is caught) precisely because landing on `.00` means the
figure was rounded, and a rounded figure whose major unit is nine identical digits is not computed.

**€88,888,888.88, 11 tenders, is the one left unresolved.** It is not a clean division either, so it
may well be a key held down — admitted for want of evidence rather than because it was cleared. Its
titles were not read, and reading them is the way to move it. Pinned as such by an assertion so the
gap is visible in the test rather than implicit.

Fixed in `7c8a443`: a nines-at-cent-level leg, the digit walk extracted as `repdigit_len` so both legs
share it, and the test's last assertion — which was `!sentinel_amount(99_999_999_999)` with the
comment "someone computed it, not typed a maximum" — inverted with the measurement in its place.

**This is the fourth time on this issue that a rule was decided, deployed, and left inert on standing
rows**, and it is worth naming as a shape rather than a run of bad luck: 372's `quality` column
(migration missing), the OTROS gate, 366 unit 1 (drained above), and now this. The remedy is the same
every time — re-fold the affected class and re-measure it — and the tell is the same: the rule's own
class still has members when you go looking.

### Still open after this firing

- **The 322-row drain** (249 major-unit repdigits the current rule already refuses but that were never
  in any cohort drained above, plus the 73 nines-in-cents the fix adds) waits on `7c8a443` reaching
  the box. **Ordering matters and the issue already learned it:** drain before the deploy and the rows
  are re-elected under the narrower rule.
- **The 24,585 exact zeros** stay an open decision. `sentinel_amount` deliberately returns false for
  zero and Leg A's reasoning for that stands (a planning notice publishes 0).
- **The detail payload and the head column disagree, and that is unit 3's unfinished half.**
  `/v1/tenders/3323836` still serves `submission_deadline = "3005-07-06"` while `status` and
  `sort=deadline` use the elected 2005-06-15; `/v1/tenders/43065` still serves
  `value {cents: 25756286172000000, currency: PLN}` while its head column is NULL. The read layer's own
  pick (`read.rs:1435-1443`, `ORDER BY s.utc_seconds DESC LIMIT 1`) was never brought onto the same
  ladder — "unit 4 is SUBSUMED" reasoned correctly about the BOUNDS, which compare the head column, and
  that reasoning does not extend to the display pick. Keeping the published figure in the payload is
  ADR-0004-faithful and may well be right; **serving it in the same field name the filters disagree
  with is not**, and that is the decision unit 3 still owes.
- **Non-EUR published sentinels are invisible to the cohort selectors** above, which read the converted
  head column: a PLN nines-run converts to a non-repdigit EUR figure. The RULE catches them (it reads
  published cents), so any row re-folded for any reason is fixed; nothing systematically hunts them.
  Section 10's sweep is the instrument that can see them.

### The repdigit drain, and the ordering read back clean (2026-09-10, rev `aa55f6e`)

**322 → 0 in one batch** — exactly the 249 + 73 predicted, which is the check that the value list and
`sentinel_amount` agree about what the rule refuses. Deployed first, drained second, per the ordering
constraint this issue recorded the last time a rule was widened.

The top of `ORDER BY current_value_eur_cents DESC` now:

| cents | tenders | title |
| --- | --- | --- |
| 10,000,000,000,000 | 2 | "SPS/CT", "Acquisition de prestations…" |
| 9,464,095,587,365 | 1 | "Strategic Partner for the Sunderland Smart City" |
| 9,406,231,628,454 | **2** | "Construction Works and Associated Services…" |
| 9,235,038,944,626 | 1 | "Modernizarea liniei CF București Nord – Jilava" |

No sentinel and no repdigit remains. **What is left at the top is the €10–100 bn band this issue
deliberately did not gate** — Sunderland (population ~275,000) is not a €94 bn smart-city programme,
and separating it from a real mega-framework needs the tender-versus-lot-sum ratio and unit 5's
`@FMTVAL` comparison, which is where the issue already put it.

Two observations for whoever takes that band, neither of them a rule:

- **9,406,231,628,454 stands on TWO tenders.** A non-round implausible value repeating exactly is the
  shape section 10's repetition detector exists to surface, and it is not a repdigit — so it is a
  sentinel family nobody has named, or it is one procurement folded into two tenders (which would make
  it a 364-shaped grouping finding instead). Worth one look before assuming either.
- **The two rows AT the ceiling** sit there because `IMPLAUSIBLE_EUR_CENTS` is compared with `<=` and
  €100,000,000,000.00 is also a round power of ten, which Leg B's decision explicitly admits. The two
  rules meet exactly there. Consistent with what was decided, and worth knowing before someone reads
  the top of the ordering and thinks the ceiling leaks.

### The "Done when" list is fully met, and the issue is NOT closeable — read this before closing it

All four acceptance items now hold: no head value above the ceiling (and the ordering re-read and
recorded above), 3323836 out of `status=open`, `?max_value=0` returning no negatives, and the
`/docs#caveats` sentence corrected (see 370 for the rewrite and why it became a rule rather than a
count).

**That list was written before the issue understood itself.** Three units it later grew are open, and
two of them are user-visible:

1. **Unit 3's read-layer half.** `/v1/tenders/3323836` still serves `submission_deadline` =
   `3005-07-06` and 43065 still serves 257 trillion PLN, in the same field names the filters now
   disagree with. The acceptance list only ever asked about `status=open`, so it cannot see this.
2. **Unit 5** (`@FMTVAL` versus element text) — the signal the €10–100 bn band needs, and that band is
   what now tops the ordering.
3. **Unit 6's magnitude-ranked listing**, and the 24,585 exact zeros.

Recorded because a "Done when" list that passes is exactly when an issue gets closed by someone
skimming, and this one's bar is narrower than its own findings.

### Unit 3 is DONE — and the drain is what made it urgent (2026-09-10, rev `45c18c7`)

Unit 3 asked to "state the deadline tie-break once … and make canonical.rs:1186 and read.rs:1440 read
the same ladder, the 343 way". Done, for the amount half as well as the deadline half, and the
sequencing is worth recording because it inverts how the unit was framed.

**The drain created the urgency rather than revealing a pre-existing gap.** Before it, the head column
and the display pick were wrong in the SAME way — both elected the junk — so they agreed and nothing
looked broken. Landing the election in the fold and then correcting ~16,500 standing rows made the
head column right and left the display pick alone, so the divergence became real on exactly the rows
the drain had just fixed. **A correctness fix applied to one of two agreeing implementations converts
a silent shared bug into a visible disagreement**, and that is a reason to look for the second
implementation as part of the fix, not afterwards.

`current_value_eur_cents`' own doc had already named the second implementation without anyone
noticing: it calls itself *"the eur_cents twin of the read layer's OLD `MAX(a.cents)`"*. The word
"old" was wishful — the head column replaced the aggregate for the BOUNDS only, while
`tender_select_head` (every list shape AND the detail payload) kept the raw extrema for display.

**Two techniques, and the difference is about drift rather than taste:**

- **The deadline horizon transcribes faithfully** — one comparison against one constant — so the SQL
  carries `s.utc_seconds - v.published_at <= {DEADLINE_HORIZON_SECS}`, interpolated from the constant
  rather than retyped.
- **The amount rule does NOT transcribe.** `sentinel_amount` is a digit walk; writing it in SQL would
  be precisely the second implementation this issue is about. So the amount pick re-derives nothing —
  it looks up the row the fold already chose, matching `s.eur_cents = t.current_value_eur_cents`.
  Zero drift by construction: change the election and this follows with no edit here.

That second technique is the transferable one. **When a rule cannot be expressed in the other
language, do not translate it — look up its result.** It needs the deciding side to persist its
answer, which the head columns already did.

**A separate incoherence fell out of it.** `MAX(a.cents)` compared raw numbers across currencies, so
1,000,000 HUF outranked 500,000 EUR, and `cents` and `currency` were independent picks that could
describe different rows. Matching on `eur_cents` ranks by value and takes both columns from one row.

**Nothing was lost from the payload.** The scalar fields are now the elected values; the `amounts` and
`dates` arrays still carry every published figure, which is where ADR-0004 faithfulness lives. Read
off prod after the deploy:

| tender | `value` | `submission_deadline` | published rows kept |
| --- | --- | --- | --- |
| 3323836 | — | **2005-06-15** (was 3005-07-06) | 2 dates |
| 43065 | **null** (was 257 tn PLN) | 2026-02-17 | 1 amount |
| 4490098 | **€50,000** (was €4.97×10¹⁶) | 2011-04-08 | 2 amounts |
| 26 | €0.01 unchanged | 2023-11-28 | 1 amount |

List timings after the change, since it touches the hot query: plain 0.79 s,
`status=open&sort=deadline&order=desc` 0.81 s, `min_value` 0.50 s, `country=DE` 0.59 s. The
subquery reads the same `(tender_id, seq)` slice the old aggregate did.

**Tests drive `apply_tenders`** — the fold's own path — so the head columns are written by
`head_value_eur_cents`/`head_deadline` themselves and the expectations are read back off them rather
than written as literals a stale pick could match by coincidence. Verified by reverting both filters:
all three go red.

**And it found a THIRD and FOURTH implementation, now issue 375.** `backfill_current_deadline` and
`backfill_current_value_eur` still write these columns with unfiltered aggregates, so running either
undoes the drain — and both docs asserted the agreement they no longer had. Filed rather than fixed
here because the disposition (retire in favour of `refold-notices`, or repair) is a real decision, and
the value half cannot be repaired in SQL for the same digit-walk reason as above.

### The cent-level leg was nines-only on a HYPOTHESIS, and the carriers refute it (2026-09-10, `5bec250`)

The nines-in-cents leg was committed with an asymmetry: nines refused, every other digit admitted,
because *"division genuinely produces a repdigit tail, so €3,333,333.33 is €10 M / 3 and
€1,111,111.11 is €10 M / 9 — computed figures, and they stay admitted"*. **That was a hypothesis
written as a finding**, and the query that settles it is one line:

| value | tenders | **distinct buyers** |
| --- | --- | --- |
| €3,333,333.33 | 26 | **17** |
| €2,222,222.22 | 7 | **7** |
| €1,111,111.11 | 12 | **10** |
| €88,888,888.88 | 13 | **1** |

Seventeen unrelated buyers do not each divide their own budget by three and land on the identical
cent. **One exact value shared across unrelated buyers and unrelated subjects is a TYPED constant** —
the repdigit rule's original argument, which never depended on which key was held down. The titles
agree: €88,888,888.88 on thermal clothing, street sweeping, snow clearing and home visits to
childminders (one buyer, so a local habit rather than a form maximum, and no more a real €88 M
procurement for it); €3,333,333.33 across road signage, street lighting, a family magazine's
distribution, meal vouchers and school cleaning.

The test comment on that class read *"admitted for want of evidence rather than because it was
cleared — the titles were not read, and reading them is the way to move it"*. This is that read, and
it moved it. **The width threshold is untouched:** eight identical digits stays admitted, pinned.

### The drain exposed two defects in ITSELF, and they are the same defect twice

**1. The value list encoded the OLD rule.** The drain selects by an explicit list of every refused
value — bounded and index-served, which is why it was built that way — but the list is a SECOND
IMPLEMENTATION of `sentinel_amount`, and the moment the rule widened the list did not. First run
after the deploy: `round 1: 0 repdigit head value(s) left`, which reads exactly like success. It was
the tool still asking the old question. Regenerated to 170 values; the same run then found 75.

That is this issue's own recurring shape, one level out: **a cohort selector that restates a rule
will drift from it, and its failure mode is a confident zero.**

**2. The selector reads the CONVERTED column; the rule reads the PUBLISHED cents.** After the drain
the count stuck at 22 across five rounds — re-folding them changed nothing, because there was nothing
to change:

| head (EUR cents) | published | currency |
| --- | --- | --- |
| 111,111,111 | 1,200,000,000 | NOK |
| 111,111,111 | 2,800,000,000 | CZK |
| 111,111,111 | 100,000,000 | GBP |

NOK 12,000,000 and CZK 28,000,000 are round real budgets that happen to convert to €1,111,111.11.
The rule admits them correctly. The selector's prefilter cannot see that, so it produced 22 FALSE
POSITIVES — the mirror of the blind spot already recorded here (a PLN nines-run converts to a
non-repdigit EUR figure and the prefilter never sees it). Fixed by keeping the converted column as a
prefilter and adding an `EXISTS` on the published cents; the corrected selector reports **0**.

So of the 75, **53 were real and are fixed; 22 were never defects.**

**A third, smaller lesson from the fix itself:** the first attempt to patch the script replaced
nothing — the escaping was wrong — and printed "selector fixed" anyway, because the transform had no
assertion. It was caught only by re-running and seeing 22 again. A patch script that reports success
without asserting its replacement count is the same silent-failure shape as the two above.

### Left unexplained, and NOT investigable with a bounded query

Seven tenders still hold a head value of exactly €1,111,111.11 from different currencies and
different published amounts. For a CONVERTED quantity that convergence is odd, and it may be nothing
— but `tender_version_amounts` has no index on `eur_cents`, so `WHERE eur_cents = …` is a full scan
and both attempts returned **408**. Per `docs/agents/prod-box-reads.md` they were not retried: the cap
bounds the wait, not the work. Recorded as an observation, not a finding, and reachable through the
weekly report's own scans rather than an ad-hoc query.

### The widened rule discriminates correctly — and my evidence for it was measured on the wrong column

After deploying the any-digit cent-level leg, the repdigit head-value census read 75 → 22. I took that
as "22 still to drain" and re-ran the drain, which reported **0 to do**. The drain was right and the
reading was wrong, for a reason this issue has already recorded once, in mirror image.

**The 22 are not sentinels at all.** Their published amounts:

| head (EUR cents) | published | currency |
| --- | --- | --- |
| 333333333 | 39,000,000.00 | NOK |
| 111111111 | 12,000,000.00 | NOK |
| 222222222 | 2,000,000.00 | GBP |
| 2222222222 | 6,000,000,000 | HUF |
| 111111111 | 28,000,000 | CZK |
| 4444444444 | 35,000,000 | GBP |

**Round published amounts whose exchange conversion lands on a repdigit.** NOK 39 M at 11.7 is exactly
€3,333,333.33; GBP 2 M at 0.90 is exactly €2,222,222.22. That is arithmetic, not a publisher holding a
key down. All 22 are non-EUR — GBP 13, NOK 3, SEK 2, HUF 2, CZK 2, **zero EUR** — and
`sentinel_amount` reads the PUBLISHED cents, so it refuses none of them. Correctly.

So the real split of the original 75: **53 were EUR-published repdigits** (where `cents == eur_cents`),
refused by the widened rule and re-elected away by the refolds that followed the deploy; **22 were
conversion coincidences** that were never candidates.

**This is the same trap the issue's own "Sweep 1" recorded, run backwards.** That sweep looked for
sentinels in `current_value_eur_cents` and could not find PLN ones, because *"the column is CONVERTED
— a PLN or HUF sentinel is multiplied by a rate before it lands there"*. Here I censused the same
converted column to evaluate a rule that governs the SOURCE column, and got the opposite error: extra
members that the rule never claimed. **A derived column cannot answer a question about the rule that
feeds it — in either direction.**

**The buyer-diversity evidence that justified widening was measured the same way**, so it mixed both
populations: the 26 tenders / 17 distinct buyers on €3,333,333.33 included some NOK/GBP coincidences.
The conclusion survives — 53 of the 75 were genuinely EUR-published and the drained outcome is exactly
those — but the specific counts in that table were inflated, and the honest version is "a large
majority of a mixed population", not "seventeen buyers typed this".

**The drain script was more correct than my reasoning about it.** Its selector carries an `EXISTS`
requiring the published `cents` to be a repdigit too, not just the head column — the discrimination
above, encoded months before it was needed and forgotten by me between writing it and re-reading it.
That guard is why the drain answered 0 while the census answered 22.

Worth keeping as the durable form: **a rule that reads published values must be measured against
published values.** The head column is where the rule's EFFECT shows, never where its population
lives.

## Unit 3's unfinished half is DONE, and verified on prod (2026-09-11)

The issue recorded this as the decision unit 3 still owed: *"the detail payload and the head column
disagree … serving it in the same field name the filters disagree with is not [right]"*, with two
named exhibits. Both were re-read on prod just now, against the deployed read layer:

| | was | now serves | the published figure |
| --- | --- | --- | --- |
| `/v1/tenders/3323836` `submission_deadline` | `3005-07-06` | **`2005-06-15`** — the elected date | still in `dates`, both rows: `2005-06-15` AND `3005-07-06T10:30:00+00:00` |
| `/v1/tenders/43065` `value` | `{25756286172000000, PLN}` | **`null`** — matching its NULL head column | still in `amounts`: `{cents: 25756286172000000, currency: PLN}` |

**So the decision resolved the way this issue hoped it would, and the resolution is checkable rather
than asserted.** The summary field agrees with what `status`, `sort=deadline` and the value bounds
do, so a reader who filters and a reader who reads one record now see the same claim. And nothing was
lost: both published figures survive verbatim in the fact arrays, which is ADR-0004's whole point and
the reason the change was safe to make.

The fix was the read layer's own pick being brought onto the same ladder as the head columns
(`read.rs`, `tender_select_head`): the deadline pick gained the horizon filter, and the amount pick
stopped taking a raw `MAX(cents)` and now follows the head column's election. That is exactly the
"never brought onto the same ladder" gap this entry named.

**Still open on this issue, unchanged:** the 24,585 exact zeros (a decision, not a defect — a planning
notice publishes 0), unit 5's gated archive read for `@FMTVAL` versus element text, and the non-EUR
published sentinels that only section 10's sweep can see.

## The exact zeros, DECIDED (2026-09-11): a 0 is a blank, and the head column must not elect one

Measured first, windowed, **0 failed windows**:

| | |
| --- | --- |
| tenders whose head value is exactly 0 | **24,647** |
| of those, the head version ALSO carries a positive amount | 613 |
| 0 is the only figure | 24,034 |

**The 613 are NOT a defect — the election is right and I expected otherwise.** Sampled: tender 852656
carries `result_value = €11,792,993` marked **`quality: withheld`**, and 68566 carries
`result_value = 999999999`, a nines-run field maximum. The election refuses both, correctly, and the
published 0 is what remains. A hypothesis that the election was losing a real figure did not survive
the data.

**Which field carries the zeros settles the question:**

| field | zero rows |
| --- | --- |
| `estimated_value` | 32,461 |
| **`result_value`** | **11,793** |
| **`framework_maximum`** | **1,408** |

**You cannot award a contract for nothing, and you cannot cap a framework at nothing.** 13,201 of
these rows sit on fields where 0 has no possible reading as a figure. `estimated_value = 0` is the
arguable one ("not estimated yet"), and it too is an absence rather than a price.

### Decided: do not elect 0

The head column is DERIVED, not stored, so this costs no fidelity — ADR-0004 is satisfied by the
`amounts` array, which keeps every published 0 exactly as it arrived. What changes is that a tender
whose only figure is 0 serves **no known value** instead of **€0**.

**The decisive argument is coherence, the same one unit 3 just settled.** `/docs` already tells every
reader *"Zero often means 'no value given', not a free tender … Filter zeros out of aggregates unless
you specifically want them."* A derived column that asserts €0 while the documentation beside it says
do not believe zeros is the payload-versus-filter incoherence again, one layer down.

**The counter-argument, recorded because it is real:** a genuinely free contract becomes
indistinguishable from an unstated one, and nothing separates them — the same shape that defeated
three discriminators on issue 364. The difference is that here the ambiguity already exists and the
column currently resolves it the FALSE way for at least 13,201 rows. Refusing is wrong less often
than electing.

**`?max_value=0` loses these 24,647**, and that is the point rather than a cost: a reader asking for
free contracts is today handed thousands of contracts that are not free.

### Next unit, with the ordering this issue already learned

Widen `sentinel_amount` to refuse 0, deploy, THEN drain the standing rows — **that order**, because
draining first re-elects them under the old rule. Same shape as the repdigit widening (`7c8a443` →
`aa55f6e`), which this issue recorded going the wrong way round once already.


## The zero drain, and what its termination guard found (2026-09-11, rev `55d239d`)

Deployed first, drained second, per the ordering this issue recorded after getting it backwards on
the repdigit widening. `55d239d` widened `sentinel_amount` with a zero leg (its own leg, not a
`repdigit_len` tweak, so the reason stays legible), flipped the two assertions that pinned the old
behaviour, and updated `/docs` and CHANGELOG from three skipped classes to four.

**The drain: 24,039 tenders, 26 rounds of 1,000, `0 zero head value(s) left`.** Script committed as
`.scratch/tender-db/drain366-zero.sh`.

The exhibit, tender 137 (`Ganzjahresstützpunkt Maulbronn`, one `result_value` of €0):

| | before | after |
| --- | --- | --- |
| `value` | `{cents: 0, currency: EUR}` | `null` |
| `amounts` | one €0 row | **unchanged** |

That pair is the whole claim: the election refuses the 0, and the payload beside it still carries it.
It works because unit 3 made the payload read the fold's election (`s.eur_cents =
t.current_value_eur_cents`, gated on NOT NULL) instead of repeating it — the two changes compose, and
neither would have been visible alone.

### The cohort selector, built to avoid this issue's own trap

The repdigit drain selected by an explicit value list, which was a SECOND IMPLEMENTATION of the rule
and drifted the moment the rule widened, reporting `0 left` on a corpus with 75. This one selects by
the OUTCOME — `current_value_eur_cents = 0` — which cannot drift, because it names the served number
the change is meant to remove. The published-cents `EXISTS` beside it is not a second rule either: it
is the other half of the same fact, and it is what makes the loop terminate.

### The guard is what found issue 378

Without the `EXISTS`, the cohort would have included 608 tenders that the zero leg does not touch and
re-folded them every round forever. **Those 608 serve a head of 0 from a published amount that is not
zero** — HUF 1.00 (547), one minor unit of RON/PLN/CZK/NOK/DKK/SEK/LTL (55), and six odd ones — which
convert to under half a cent and round to zero.

So the derived column can still say €0, by rounding rather than by election, and `?max_value=0` now
returns exactly those 608: a reader asking for free contracts is handed contracts priced at one
forint. Filed as **issue 378**, with ADR-0010's rounding amendment read as the precedent and a floor
at one cent as the candidate fix. Smaller wrong answer than the 24,039, same kind.

## Closed on re-read (2026-09-12)

The "Still open after this firing" list of 2026-09-10 is empty now, item by item: the 322-row
repdigit drain ran clean after the deploy (`aa55f6e`); the 24,585 exact zeros stopped being a decision
when the zero leg landed and the drain ran 26 rounds to `0 left` (`55d239d`); unit 3's unfinished half
put the payload on the fold's election and was verified on prod (2026-09-11); and non-EUR published
sentinels are what section 10 sweeps for (issue 380, with 381's per-unit-rate class read off it).
The follow-ons that grew out of this issue have their own files (378, 379, 380, 381), all DONE. Closed.
