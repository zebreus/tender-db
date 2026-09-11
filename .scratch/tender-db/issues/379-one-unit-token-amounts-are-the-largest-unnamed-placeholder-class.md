# 379 — a published 0.01 or 1.00 is a token, and 112,244 Tenders serve one as their value

Status: **DONE 2026-09-11** — units 1 and 2 shipped and drained the same day (rule `bfe7ac6`,
112,229 Tenders drained in 114 rounds, 0 left, both spikes gone from the distribution). Unit 3 (separate
the unit-price reading) is optional and stays open; see "Shipped and drained" at the end. Was:
ready-for-agent (filed 2026-09-11 by the owner, found while measuring issue 378's HUF 1.00
class — the measurement went one query wider than the issue asked and hit something much larger)
Kind: defect (derived values) — **the largest placeholder class in the corpus, and it has never been
named**: 4.7× the exact zeros just drained, 7× the negatives
Blocked by: nothing

## The finding

Issue 378 asked a narrow question: is HUF 1.00 a publisher token? The answer is yes, and the same
query pointed at every other currency says the convention is not Hungarian and not about HUF.

**Head-value distribution at the bottom of the scale** (`tenders.current_value_eur_cents`, indexed,
measured 2026-09-11):

| head (EUR cents) | Tenders |
| --- | --- |
| **1 (€0.01)** | **59,030** |
| **100 (€1.00)** | **53,214** |
| 10 (€0.10) | 1,463 |
| 200 (€2.00) | 658 |
| 13 | 562 |
| 4 | 499 |
| 9 | 495 |

Two spikes and then nothing. The third-placed value is **36× smaller** than the second, and the tail
below it (13, 4, 9, 11, 117, 111 …) is foreign-currency conversions landing on arbitrary cents, not a
convention. **112,244 Tenders serve a head value of exactly €0.01 or €1.00.**

The spike is in the PUBLISHED figure, not the conversion. Per currency, rows at exactly one major
unit versus rows at two:

| currency | Tenders at 1.00 | rows at 1.00 | rows at 2.00 | ratio |
| --- | --- | --- | --- | --- |
| EUR | 65,034 | 144,604 | 1,155 | 125× |
| HUF | 2,580 | 6,047 | 28 | 216× |
| CZK | 1,234 | 5,111 | 7 | 730× |
| SEK | 2,302 | 3,030 | 21 | 144× |
| DKK | 1,671 | 2,470 | 6 | 412× |
| GBP | 1,126 | 2,362 | 28 | 84× |
| NOK | 1,074 | 1,437 | 9 | 160× |
| BGN | 75 | 260 | 63 | 4× |
| PLN | 129 | 220 | 22 | 10× |
| RON | 30 | 172 | 18 | 10× |

Ten currencies, every one of them spiking at exactly one unit. **A distribution does not do that. A
typed constant does** — which is the argument that convicted the repdigit field maxima, in a
different part of the number line.

## Which field carries it, and why that settles it

| | `result_value` | `estimated_value` | `framework_maximum` |
| --- | --- | --- | --- |
| EUR 0.01 (rows / Tenders) | **79,846 / 64,065** | 17,985 / 8,555 | 634 / 196 |
| EUR 1.00 (rows / Tenders) | **69,405 / 53,578** | 72,933 / 25,029 | 2,266 / 585 |
| HUF 1.00 (rows / Tenders) | 5 / 4 | **6,037 / 2,575** | 5 / 2 |

`result_value` dominates both EUR classes. **You cannot award a contract for one cent**, and the same
reasoning that carried the zero decision carries this one, with a larger population behind it.

HUF is the interesting exception: there the token sits on `estimated_value` (99.8%), which reads as
"no estimate stated" rather than "award value withheld". Same token, different field, same meaning.

## The carriers, and the two adversarial checks

- **780 distinct buyer organizations** across the 1,899 one-cent Tenders in the `id < 300000` slice.
  (The whole-cohort version of this query exceeds the 10 s cap; the slice is a sample and is named as
  one. It contains the phenomenon — 1,899 of the 59,030 — per `docs/agents/prod-box-reads.md`.)
  780 unrelated buyers do not each award a contract for a cent.
- **1,620 distinct buyers** across the 2,580 HUF 1.00 Tenders (whole cohort, no sampling).
- **Source: TED 111,131, DOE 1,109, FTS 4.** Not one platform's quirk, unlike issue 377's Swiss
  publisher.
- **Ten years and still current**: the HUF class runs 2016-05-07 → 2026-09-09.

**Adversarial check 1, the concession reading — FAILS to explain it.** A €1 symbolic sale is a real
legal device, so the class could in principle be concessions and land transfers. It is not: in the
`id < 300000` slice the notice subtypes are **29 (3,383), 16 (388), 33 (231), 30 (223)** — ordinary
award notices, the same subtypes issue 376 found for the revenue-side negatives, with no concession
marker. Symbolic sales exist; 112,244 of them do not.

**Adversarial check 2, the subjects — FAILS to explain it.** Read from the API at
`?min_value=1&max_value=200`: a gymnasium general renovation and extension, Deutsche Bundesbank site
security in Frankfurt, a Max Planck Institute new building's painting works, fire-brigade
defibrillators, Microsoft licences, a day-care new build's carpentry, interim award of school meals,
fire-protection planning. These are ordinary works and services procurements. None of them costs a
cent.

## The one reading that survives, and why it does not change the decision

**A unit price.** Some publishers put a per-unit figure in a value field — €1.00 per meal, per litre,
per licence — and that is a real number honestly published. It would explain part of the 1.00 class
(and fits `estimated_value` better than `result_value`), and I have not separated it out.

It does not change what the head column should do. **Whether the 1.00 is a token for "not stated" or
a unit price for one item, it is not the procurement's value**, and a derived column that asserts it
as the Tender's headline is wrong under either reading. The `amounts` array keeps the published
figure either way (ADR-0004), so nothing is lost by refusing to elect it.

## Decision: refuse an exact 0.01 and an exact 1.00, as EXACT values

This fits `sentinel_amount`'s design rather than straining it. The function refuses **exact values,
never magnitudes** — deliberately, and its doc says so — because a magnitude floor would delete
genuine small procurements to catch a convention. One minor unit and one major unit are two exact
values, the same shape as −1.00 and the same shape as 0:

```rust
if cents == 1 || cents == 100 { return true; }
```

**€0.10 and €2.00 stay admitted.** 1,463 and 658 Tenders, 36× and 80× below the spikes — that is a
tail, not a convention, and extending the rule to reach them is where an exact test would become a
threshold.

**The counter-argument, recorded because it is real:** a genuine €1 symbolic contract and a genuine
€0.01 unit price both become invisible, and nothing in the data separates them from the token. The
same trade the zero decision made, at 4.7× the scale — which is a reason to state it plainly, not a
reason to decide differently. Refusing is wrong far less often than electing.

## Units

1. **The rule**: the two exact legs in `sentinel_amount`, with the spike table as the doc comment's
   evidence, tests pinning that €0.10 and €2.00 stay admitted, and `/docs` + CHANGELOG updated from
   four skipped classes to five.
2. **The drain**: 112,244 Tenders, so ~115 rounds of `refold-notices` at the 1,000 cap — about two
   hours. `.scratch/tender-db/drain366-zero.sh` is the template; select by the OUTCOME
   (`current_value_eur_cents IN (1, 100)`) with the published-cents `EXISTS` guard, for the reasons
   that script's header gives. **Deploy first, drain second** — issue 366 recorded that ordering the
   hard way twice.
3. **Separate the unit-price reading** if it is worth knowing: the 1.00 rows on `estimated_value`
   whose notice also carries a quantity. Not a blocker for units 1 and 2.

## Relationship to issue 378

378 is the *rounding* road to a derived zero — 608 Tenders, HUF 1.00 and one-minor-unit amounts that
convert to under half a cent. This issue is the *election* road, and it is 185× bigger. 378's "next
unit" (measure the HUF 1.00 carriers) is answered here and should point at this issue. **The two fixes
are independent**: refusing an exact 1.00 stops it being elected, and flooring the conversion at one
cent stops a surviving amount converting to a derived 0. Both are needed for the head column to mean
one thing.

## Shipped and drained (2026-09-11, rev `bfe7ac6`)

Deployed first, drained second, per the ordering issue 366 recorded the hard way twice.

**Unit 1, the rule.** Two exact legs, `cents == 1 || cents == 100`, with the spike table as the doc
comment's evidence. `one_unit_is_a_token_and_the_values_beside_it_are_not` pins the boundary that
matters — €0.10 and €2.00 stay admitted — so a later edit cannot quietly turn a measured spike into a
magnitude threshold. `/docs` and CHANGELOG went from four skipped classes to five.

One assertion had to be flipped **three hours after it was written**. The zero test carried
`assert_eq!(value(vec![amount(1)]), Some(1))` with the comment *"a €0.01 award is implausible, but
nothing here measures plausibility and a magnitude rule is the thing this function keeps refusing to
become"*. That was right about the METHOD and wrong about this value, and the replacement says which:
0.01 turned out to be an exact typed constant on 59,030 Tenders, which the exact design reaches
without becoming a threshold. The comment defended a principle the evidence never threatened.

**Unit 2, the drain.** 112,229 Tenders, 114 rounds of 1,000, `0 one-unit head value(s) left`. Script
committed as `.scratch/tender-db/drain379-token.sh`.

**The distribution afterwards**, which is the verification that matters — both spikes are gone and
what was underneath them is untouched:

| head (EUR cents) | before | after |
| --- | --- | --- |
| 1 (€0.01) | 59,030 | **0** |
| 100 (€1.00) | 53,214 | **0** |
| 10 (€0.10) | 1,463 | 1,504 |
| 200 (€2.00) | 658 | 658 |
| 13 | 562 | 562 |

**€0.10 went UP by 41**, and that is the refusal working rather than a leak: 41 Tenders that used to
elect a token now elect the next amount down, which happens to be €0.10. The election falls through;
it does not empty the column by reflex.

The exhibits, all three serving `value: null` with their published figure intact in `amounts`:

| tender | subject | published |
| --- | --- | --- |
| 930 | Wolfgang-Borchert-Gymnasium Langenzenn, general renovation and extension | `result_value` €1.00 |
| 994 | Objektschutz for Deutsche Bundesbank sites in Frankfurt | `result_value` €1.00 |
| 26 | REZ SW AsAflex | `result_value` €0.01 |

A gymnasium's general renovation did not cost one euro, and the Bundesbank's site security did not
either. Both now say so.

### What this issue's instrument gap became

The discovery sweep could never have found this class: it searches negatives and everything above
€100 bn, and declines the middle and bottom on the stated ground that "frequency alone would not
identify one there". This class was found by a frequency ranking at the bottom, with a 36× cliff.
Filed as **issue 380**, which carries the refutation and why the state-blow-up argument behind the
floor does not reach a bounded low-end arm.
