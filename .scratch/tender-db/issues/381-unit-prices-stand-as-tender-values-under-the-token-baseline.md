# 381 — a per-unit rate stands as the tender's value, and the new bottom-of-range sweep found it on its first run

Status: **DONE 2026-09-11** — unit 1 shipped (`2130cf4`, verified against the prod numbers), units 2
and 3 answered: there is NO per-unit signal to gate on, so the class is DOCUMENTED rather than
filtered, and the harm is 8,951 Tenders. See "Closed" at the end. Was: ready-for-agent (filed
2026-09-11 by the owner, from the first run of issue 380's low-end sweep — job 1305, 7,051 s, 0
labels unmeasured)
Kind: defect (derived values) — the class issue 379 recorded as "the one reading that survives" and
did not separate out, now measured and exhibited
Blocked by: nothing

## The exhibit

Three GBP tenders, read from the API after the listing pointed at them:

| tender | title | served `value` |
| --- | --- | --- |
| 6035731 | Taxi Vehicles with Passenger Assistance | **£8.57** |
| 6109876 | Taxi Vehicles | **£8.57** |
| 6113040 | Taxi Vehicles | **£8.57** |

£8.57 is a **per-journey or per-mile taxi rate**, published into `estimated_value` AND `result_value`
identically. It is a real number, honestly published, and it is not the value of the procurement. The
head column asserts it as one.

This is exactly the reading issue 379 recorded as surviving its evidence — *"a per-unit price honestly
published in a value field … It is not separated out"* — and 380's sweep separated it out, on the
first run, by putting it in a ranked listing under the token baseline.

## How big, and where

The residual after removing the three classes the election already refuses (0, 0.01, 1.00), as a
share of each currency's amount rows:

| currency | residual rows ≤ 10.00 | share of that currency's rows |
| --- | --- | --- |
| **BGN** | 11,877 | **0.829 %** |
| **GBP** | 2,225 | **0.587 %** |
| EUR | 18,121 | 0.185 % |
| DKK | 212 | 0.120 % |
| RON | 4,341 | 0.089 % |
| SEK | 250 | 0.084 % |
| NOK | 100 | 0.067 % |
| PLN | 1,000 | 0.028 % |
| CZK | 189 | 0.016 % |
| HUF | 42 | 0.015 % |

BGN's residual is **4.5× EUR's and 55× HUF's**. Its shape is a small-integer ladder on
`estimated_value` — 10.00 (655 rows / 42 Tenders), 8.00 (327/27), 5.00 (320/30), 6.00 (294/30),
4.00 (208/22) — with non-round neighbours (8.33, 7.50, 8.58) that read as division results. GBP's is
the same shape: 10.00, 5.00, **8.57**, **4.29**, each appearing on `result_value` and
`estimated_value` with identical counts.

8.57 ≈ 60/7 and 4.29 ≈ 30/7. These are rates, not prices.

## The measurement sequence, because two readings of it were wrong first

This is worth recording as method, not just result. **Three questions, three different answers, and
only the third is right.**

1. **Raw count in the listing** — BGN takes 8 of the 40 slots, more than any currency but EUR. Reading
   this as "something is wrong with BGN" is a **volume artifact**: BGN has 1,432,752 amount rows.
2. **Raw rate** — BGN's rows ≤ 10.00 are 1.55 % of its total, *below* EUR's 4.07 %. Reading this as
   "BGN is unremarkable" is a **baseline artifact**: EUR's 4.07 % is almost entirely the 0/0.01/1.00
   tokens, which are known and already refused.
3. **Residual rate** — remove the three known classes and BGN leads at 0.829 %, EUR falls to 0.185 %,
   and the ordering inverts completely.

The raw count and the raw rate disagree with each other AND both disagree with the truth. The
listing as built cannot show any of this, because it ranks by count within one currency-blind
ordering.

## Units

1. **Give the low listing a residual rate per currency.** Not by filtering the known classes out of
   the listing — issue 380 argued against that and the argument stands — but by reporting, beside it,
   each currency's rows ≤ ceiling excluding the values `sentinel_amount` refuses, over its total rows.
   That is the column that makes the regions comparable, and without it the listing invites exactly
   the two wrong readings above. **The exclusion list is the drift risk 380 named**, so derive it from
   one place: a single `KNOWN_REFUSED_EXACT` constant the rule and the report both read, rather than
   two literals.
2. **Decide what to do about the class itself.** Unlike the tokens, this is NOT reachable by the exact
   design: 8.57 and 8.33 are not typed constants, and refusing them needs either a magnitude floor —
   which `sentinel_amount` refuses by construction, for good reasons stated in its doc — or a signal
   the number does not carry. Candidates worth checking before any rule: whether eForms publishes a
   per-unit indicator alongside the amount, and whether the legacy forms do; whether a lot-level
   quantity exists to multiply by. **If no signal exists, the honest outcome is to document the class
   rather than guess at it**, the way issue 376 documented the revenue-side negatives.
3. **Size it in Tenders, not rows.** The table above counts amount rows. The user-visible harm is
   Tenders whose HEAD value is one of these, which is a smaller number and the one that belongs in
   any decision.

## What this says about issue 380's instrument

It worked, first run, exactly as argued: the baseline sat at the top, labelled and explained, and the
new class was directly under it. The gap it has is unit 1 above — the listing shows counts where the
question needs rates.

## Closed (2026-09-11)

### Unit 1 — shipped and verified against prod

The residual share landed in `2130cf4` and the next report run reproduced, from independently
written SQL, every number this issue was filed with: BGN 11,877 / 1,432,752 = 0.829 %, GBP 2,225 /
379,052 = 0.587 %, EUR 18,121 / 9,807,417 = 0.185 %, and DKK / RON / SEK / NOK / PLN / CZK / HUF all
matching to the row. Two implementations agreeing is the check that the column measures what the
hand-rolled queries measured.

It also surfaced what hand-picking ten currencies had missed: LVL 0.338 % and FRF 0.281 % rank above
EUR, on 31 and 35 residual rows. And the pre-euro currencies (ATS, ESP, ITL, PTA, SKK, MDL) sit at
**0.000 %** — the habit is a modern-era one, which no count-ranked listing could have shown.

### Unit 3 — the harm, in Tenders

**8,951 Tenders serve a head value at or under €10**, after the token drain removed 112,229:

| band | Tenders |
| --- | --- |
| ≤ €0.10 | 2,892 |
| ≤ €1 | 1,304 |
| ≤ €10 | 4,755 |
| ≤ €1,000 | 30,191 |

**There is no cliff.** The bands rise again above €10, which is the opposite of the 36× gap that
convicted the tokens. A distribution with no step has no honest threshold in it.

### Unit 2 — DECIDED: no signal exists, so document rather than rule

The issue said the decision turned on whether a per-unit indicator is available. It is not, on two
independent counts:

1. **The source does not publish one.** eForms amounts carry `@currencyID`. `@unitCode` exists in the
   schema but belongs to the `measure` type, which is how DURATIONS are expressed (value + DAY /
   MONTH / YEAR) — not amounts. There is no field that says "per journey".
2. **The model could not hold one anyway.** The dropped-vocabulary list in `project.rs` records
   `UBL-ExpectedOperatorQuantity` as *"integer; no integer fact channel"*, and the same for number
   and code fields. A quantity to multiply by has nowhere to land.

So refusing this class would need a magnitude floor, which `sentinel_amount` refuses by construction
and which the band table above shows would be arbitrary. **Documented instead**, the way issue 376
documented the revenue-side negatives: `/docs` now tells readers that a very small value may be a
per-unit rate, gives the taxi exhibit and the band counts, and says plainly that these are NOT
filtered and why.

**What would reopen this**: a source that does mark per-unit amounts. FTS and any future source
should be checked for one when they are mapped — that is a cheaper question at mapping time than a
retrofit.
