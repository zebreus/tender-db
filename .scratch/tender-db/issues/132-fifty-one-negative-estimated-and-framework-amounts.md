# 132 — the 51 negative `estimated_value` / `framework_maximum` rows the re-spec did NOT explain

Status: open — **CANONICAL** for this follow-up; issue 160 is a pointer here. The residual left by issue
33's re-spec. **Not noise, and deliberately not swept under the narrowed invariant.** Small enough to
inspect row by row.
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

## Merged from issue 160 (sdk-vendor's triage-side evidence)

We filed this same follow-up within minutes of each other from opposite ends of #33 — I from the fix
side, sdk-vendor from the triage side — and then each deferred to the other, briefly leaving two issues
pointing at one another and none canonical. Resolved: **132 is canonical, 160 is the pointer.** Their
evidence, kept so it is not lost with the number:

**Already established, so nobody re-derives it:**

- **Not the fold** — P4 = 0 and 17,738/17,738 exact magnitude matches against the chain's parse layer.
  *But those are whole-set figures*, dominated by the 17,687 `result_value` rows; whether these 51
  individually satisfy P4 is the open question below, and the aggregate does not answer it. (sdk-vendor
  caught themselves making that over-read in their own file while deduping.)
- **Not a parser-version defect** — the full set scatters across 13 SDK versions (1.6→1.14 plus DE) and
  4 years. A version-specific mapping fault would cluster.
- **Magnitudes differ from the benign mass** — €420k and €11.8M scale, against a `result_value`
  population that is 99.6 % between €1 and €100. These do not look like adjustment lines.

**The open questions, sdk-vendor's third being the one to chase first:**

1. Do the 51 cluster on profile/version/publisher *once isolated from the 17,687*? Whole-set scatter
   does not rule out a cluster inside the subset, and nobody has looked.
2. Is a negative `estimated_value` a published **sentinel** rather than a value? The recurring
   exactly-`-100` (−1.00) pattern across several currencies suggests sentinels exist in this data.
3. **Is some field whose published semantic is a DELTA being mapped as an absolute ceiling?** Invisible
   from the sign alone, and the hypothesis that best explains why a *ceiling* — which cannot meaningfully
   be negative — carries one. My area; this is where I would start after P4.

**Framing worth keeping verbatim** (team-lead): *17,738 unexplained → 51 worth explaining is progress,
not resolution.* The failure it guards against is using a mostly-benign explanation to wave through the
part it does not cover.
