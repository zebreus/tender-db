# 132 — the 51 negative `estimated_value` / `framework_maximum` rows the re-spec did NOT explain

Status: **SUPERSEDED by issue 160** (sdk-vendor), which covers the same 51 rows with more evidence —
magnitudes (EUR 420k and 11.8M scale, unlike the EUR 1-100 mass), the exactly-`-100` sentinel pattern, and
the within-subset clustering question. We filed the same residual independently within the hour; theirs is
richer, so this one stands down. **Read 160.** Kept only for the one question of mine it does not carry
(P4 over the 51), noted below.
Kind: data quality
Owner: unassigned (proj-fix filed; the parse-vs-fold half of 33 was mine)
Relates to: 33 (the triage + re-spec), 131 (the parse-vs-fold determination), `run_light` 3.7,
`standing_gate` `negative_amount`

## What is left

Issue 33 established that negative amounts are published by the source and faithfully copied by the fold
(P4 = 0; 17,738/17,738 exact magnitude matches against the chain's parse layer). 3.7 was re-specified to
permit negatives on `result_value` — 17,687 of the 17,738, award/result adjustments, 99.6% between EUR 1
and 100.

That leaves **51 rows the explanation does not cover**:

| field | rows |
|---|---|
| `estimated_value` | 29 |
| `framework_maximum` | 22 |

A negative *result* value is an adjustment and makes sense. A negative **estimated** value, or a negative
**framework maximum**, does not: an estimate of what a procurement will cost, and a ceiling on what may be
spent under a framework, are quantities with no meaningful negative reading.

## Why this is filed rather than closed with 33

The re-spec narrows *17,738 unexplained* to *51 genuinely-suspect*. That is the whole value of narrowing
rather than relaxing — the check now points at something specific instead of at everything or nothing.
Letting these 51 disappear into a passing gate would reproduce, at 1/350th the size, exactly the failure
issue 33 exposed: an invariant that stops telling anyone anything.

They are also **where a real defect would live**. Issue 33's conclusion (the source publishes them, both
layers are faithful) was established over the whole 17,738 and is dominated by the `result_value` mass. It
does not follow that it holds for these 51 — a mapping fault putting the wrong field's value into an
estimate, for instance, would look exactly like this and would be invisible in the aggregate.

## What would settle it

Small enough to inspect row by row (51 rows), which is the luxury the narrowing bought:

1. Do these 51 also satisfy P4 — an equal-magnitude negative in the chain's parse layer? (If not, the fold
   half of issue 131 needs re-opening for this subset.)
2. What are their source notices, profiles, and magnitudes? A cluster on one profile or SDK version points
   at a mapping fault; a scatter points at genuinely odd publications.
3. Do the published documents actually carry a minus on that field, or is a sign arriving from somewhere
   else (an unmapped element, a delta mistaken for an absolute)?

Question 1 is cheap and decisive about *our* code; 3 is the one that needs a human reading a notice.


---

## Superseded — the one thing to carry across to 160

sdk-vendor filed issue **160** for these same 51 rows, independently and within the hour, with more
evidence than this file has. That is the second same-day collision between us (the first was issue 121),
and it is what a tracker with no allocator and no claim step produces when two people work the same finding
from different ends.

The one question here that 160 does not already carry, and the cheapest of the set:

> **Do the 51 satisfy P4?** — for each, does the version chain's parse layer hold an equal-magnitude
> negative? Issue 33 established this across all 17,738, but that result is dominated by the 17,687
> `result_value` mass; it does not automatically transfer to a 51-row subset. If any of the 51 fails P4,
> the fold introduced a sign for them specifically and issue 131's exoneration needs re-opening for this
> cohort. Decisive about our own code, and nearly free once the query is pointed at the subset.

Everything else in this file is said better in 160.
