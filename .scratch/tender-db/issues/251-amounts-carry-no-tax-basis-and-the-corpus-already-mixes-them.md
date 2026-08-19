# 251 — amounts carry no tax basis, and the corpus already mixes inclusive with exclusive figures

Status: needs-triage, filed 2026-08-19 (owner) — surfaced by issue 244's value slice, but it is NOT a
text-era problem: it is corpus-wide and predates that work
Kind: canonical modelling gap (a published qualifier with nowhere to land)
Blocked by: —
Relates to: 244 (the slice that surfaced it), 232 (the near-zero value columns), 171 (value-domain
profile), 177 (r208 values), ADR-0010 (amounts as integer cents)

## What

`tender_version_amounts` records `(kind, cents, currency)`. It has no field for whether the figure is
**inclusive or exclusive of tax**, and the sources say so explicitly:

- the text era writes it into the value itself —
  `Price: Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.` (excl.) beside
  `Price: Auftragssumme (mit Umsatzsteuer): 110 761,16 DEM.` (incl.), in the same package, same day;
  French bodies use `TTC` (incl.) and `HT` (excl.);
- the r208/r209 form eras publish a **VAT indicator** alongside the value, which this project does not
  map either.

So the column already holds both bases, unlabelled, with no way for a reader to tell which is which. A
sum or an average over it is meaningless in a way nothing warns about.

## Why it is being filed now

Issue 244's value slice refused every text-era value that stated its basis, on the reasoning that
mixing bases silently would be a subtle wrong. Then the measurement came in: that refusal drops **most
of the era's money** — six of eight sampled refusals are exactly this shape. Refusing is not neutral,
it is a large silent data loss chosen to avoid a smaller silent inaccuracy that the corpus already has
everywhere else.

That trade should be made deliberately and in one place, not per era by whoever is writing an extractor
that afternoon.

## Options, cheapest first

1. **Record it.** Add `tax_basis TEXT` (`incl` | `excl` | NULL for unknown) to
   `tender_version_amounts`, populate it where the source states it (text-era prose, the form eras'
   VAT indicator), and leave NULL where it does not. Additive migration, no reader breaks. The honest
   minimum: readers can filter, and the unknowns are visible as unknowns rather than as agreement.
2. **Surface it.** Add a data-quality report line counting amounts by basis, so the mix stops being
   invisible. Cheap, and it makes option 1's value measurable.
3. **Do nothing and document.** State in `docs/` that the column mixes bases. Costs nothing, fixes
   nothing, and at least stops a reader trusting a total.

Option 1 plus 2 is the recommendation. Until it lands, issue 244's slice 5 should claim the values and
say so, rather than discard the era's money for a defect it did not introduce.

## Acceptance

- A decision recorded here, with a reason.
- If option 1: the column exists, the text era and the form eras populate it where stated, and a test
  pins one of each from a committed fixture.
- If option 2: the report line exists and its first reading is recorded here — how much of the corpus
  is inclusive, exclusive, and unknown.
