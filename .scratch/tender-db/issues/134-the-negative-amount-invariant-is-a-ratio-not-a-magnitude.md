# 134 — the negative-amount invariant is a RATIO, not a magnitude

Status: RATE-FORM LIVE / row-level re-spec remains with the 33/37 line
(owner sweep 2026-08-28). The scale-free RATE this issue argued for ships in
the weekly DQ: section 8 measures the negative-amount rate per era every
Sunday (data_quality.rs cites 131/132/134/136 — "a parser fabricating
negatives, or a source shipping garbage at scale, moves the rate; individual
negatives do not"), with headline gauges on /metrics. What stays open here is
only the ROW-LEVEL correction invariant (negative small relative to the
positive it adjusts) for check #37's re-spec — the parameter must come from
row-level data per the original owner note. Was: DESIGN — **the delta hypothesis below is FALSIFIED (issue 136); the ratio form survives.** The
form of the re-spec for #37 (`bids.cents`, `awarded_cents`) and, on the same
argument, for #132's 51. **Proposes the shape of the invariant, not the number**: the parameter has to
come from the row-level data, which sdk-vendor holds.
Kind: data quality / verification
Relates to: 33 (`run_light` 3.7's re-spec), 36 (the ~72K finding), 37 (the re-spec), 131, 132

## The question

#36 settled the ~72K negative bids/awards as legitimate source-published corrections — 99 % at €1–100,
the same shape as the 17,687 `result_value` rows. It leaves **407 rows ≥ €10,000**, extreme at
**−€323M**, which the €1–100 explanation plainly does not cover. So the re-spec must **narrow, not
flatten**: permit the corrections, keep catching the tail.

The obvious form is an absolute magnitude bound — permit negatives under €X. **I think that is the wrong
shape, and wrong in both directions at once.**

## Why an absolute threshold is wrong both ways

A correction adjusts *something*. Whether one is plausible depends on **what it is adjusting**, not on
its own size:

- **−€5,000 on a €10,000 tender** is a 50 % adjustment. An absolute €10K bound **passes** it. It should
  not — that is not a correction, it is a restatement.
- **−€15,000 on a €50M framework** is 0.03 %. The same bound **fails** it. It should not — that is
  exactly what a correction looks like at that scale.

So an absolute number admits suspicious rows on small tenders and cries wolf on ordinary ones on large
tenders, and the error grows with the corpus's spread of contract sizes. Picking the number more
carefully does not fix it, because **the quantity being bounded is the wrong quantity.**

## The shape that encodes "is this shaped like a correction?"

**A negative amount is plausible when it is small relative to the positive amount it adjusts on the same
Tender.** That is what "correction" *means*, it is scale-free across a corpus spanning four orders of
magnitude, and it needs no number chosen by taste:

```
|negative| / (that Tender's positive amount of the same kind)  <  R
```

with `R` derived from the measured distribution rather than guessed — the value at which the population
stops looking like adjustment lines and starts looking like restatements. **sdk-vendor holds the data
that sets `R`; this note only argues for the form.**

Two cases fall out of the form rather than needing separate rules:

- **A negative with no positive to adjust on that Tender** — a correction to nothing. Suspicious by
  construction, and an absolute bound cannot express that at all.
- **−€323M** — almost certainly fails on ratio *and* on absence-of-a-counterpart, which is the kind of
  agreement between independent readings that suggests the form is right.

## Cost, stated because it decides the tier

A ratio needs the Tender's other amounts, so this is a **join**, not a single-table scan: it belongs in
**Tier B (34 min)**, not Tier A. Worth it — but it means the negative-money invariant would move tiers,
and a cheap absolute floor could stay in Tier A as a coarse backstop (nothing may be more negative than
the largest legitimate contract in the corpus). Layered, the cheap check catches the absurd and the
ratio catches the merely implausible.

## What I am NOT proposing

**A number.** #33's re-spec worked because the field split came from measured behaviour (17,687
`result_value` at €1–100 versus 29 + 22 ceilings), not from judgement. The same discipline applies here:
I can argue the invariant should be a ratio; only the data can say what ratio, and only the 407-row
investigation can say whether they are legitimate large corrections or a real defect. **If they turn out
to be a delta-mapped-as-absolute fault — issue 132's third hypothesis — then the right fix is in the
mapping and no threshold of any shape should be tuned to admit them.**


---

## The delta hypothesis is falsified — for all 183, not weakened (2026-08-05)

sdk-vendor's 407 read (issue 136) settles the sequencing risk I raised, in the direction that removes it:

| | |
|---|---|
| negatives whose lot_result has **no earlier value at all** | **92 / 183** |
| of the 91 with a prior: **plausible deltas** (`prior + neg ≥ 0`) | **0** |
| of the 91: the negative **exceeds** its prior | **91** |

**Not one row is consistent with a delta mapped as an absolute.** Half have nothing to apply a delta *to*
— impossible for those rows, not merely unsupported — and the other half would have to revise away more
than was ever there. 175 of 183 carry an **exact published magnitude** in the chain's parse layer, and
183/183 co-occur with a negative bid on the same version, so this is **one finding seen twice**: fix the
source handling for bids and awarded follows.

**So the risk I flagged is gone.** I had argued: investigate before re-speccing, because a threshold tuned
to admit a mapping fault would be *narrowed rather than flattened* and still stop catching the thing. The
investigation happened and there is no mapping fault to accidentally admit. **#37 can proceed.**

**But the threshold must still not be derived to admit them**, for a reason I had not separated:
*source-published* is not *correct*. −€323,093,120.12 is implausible on its face. What the read
establishes is that it is **faithfully carried** — not our defect — which is the same distinction we held
to for issue 84's 154 (*held-but-unextracted* ≠ *lost*). A bar tuned to let these through would be
calibrated to accept **published garbage**: a different error from the one this issue feared, and still an
error. They stay flagged as a **source-quality** finding, and the ratio's parameter comes from the benign
mass alone.

### Q5 does not change the ratio's shape — recorded so nobody re-runs it on my account

Q5 (profile/era clustering of the residue) never ran; the pass hit its 2400 s bound. It is worth having
for the source-quality finding — *is this one publisher's convention or scattered noise* — but it does
**not** change this design:

- the parameter is set by the **benign** population, not the residue, so how the residue clusters cannot
  move it;
- and if the residue *were* one publisher, a global ratio would flag that publisher permanently — which
  under #28's state-not-event rule is a **standing condition, reported and not alerted**, not the
  alert-fatigue failure that would otherwise force a profile-scoped rule.

So the answer to "would Q5 change the threshold's shape" is **no**. Run it if the source-quality finding
needs it; not for this.
