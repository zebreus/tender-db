# 134 — the negative-amount invariant is a RATIO, not a magnitude

Status: DESIGN — the form of the re-spec for #37 (`bids.cents`, `awarded_cents`) and, on the same
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
