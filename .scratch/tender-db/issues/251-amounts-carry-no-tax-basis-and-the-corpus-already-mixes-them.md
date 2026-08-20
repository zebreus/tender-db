# 251 — amounts carry no tax basis, and the corpus already mixes inclusive with exclusive figures

Status: OPTION 1 BUILT 2026-08-20 (owner) — `tender_version_amounts.tax_basis` exists, nullable, and
the text era populates it; the r208/r209 VAT indicator is the named follow-up, and option 2 (the
report line) is a separate unit with its own cost measurement. AWAITING DEPLOY
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


---

## Decision and what landed (2026-08-20)

**Option 1, with a nullable column.** The reason, stated plainly: refusing to record a figure because
its basis has nowhere to go is a *larger* silent loss than recording it unlabelled — issue 244's slice 4
measured that, and slice 5 reversed it. So the column exists, and the honest default is NULL.

    tender_version_amounts.tax_basis TEXT     'incl' | 'excl' | NULL when the source did not say

Every pre-existing row answers NULL, which is the truthful reading: those figures were written without
anyone knowing which basis the source called them. **NULL now reads as unknown instead of as
agreement**, which is the whole point.

### How the basis travels

As a **sibling code in the same section**, not as a field on `NoticeValue::Amount`. Widening that enum
would touch every parser and every value table; a companion field id costs nothing and is already how
the text era emits it (`TED-VAL_TOTAL_TAX_BASIS`, issue 244 slice 5). The projection builds one map of
section → basis before its value loop, so pairing is a lookup rather than a rescan per amount.

The vocabulary is closed at `incl`/`excl`: any other code is dropped rather than written, so a typo or
a future third value cannot become something readers have to guess at.

### Scope, and what is deliberately NOT in it

- **r208/r209 are not paired yet.** Both eras' VAT elements already reach the parse layer, but
  `EXCLUDING_VAT` is a presence flag and `INCLUDING_VAT` a container — neither is a code carrying
  `incl`/`excl`, so pairing them needs a payload read of how those elements sit relative to the value
  element. Named here as the follow-up; the mechanism is in place for them to join.
- **Option 2, the report line, is a separate unit.** `SELECT tax_basis, COUNT(*) …` over
  `tender_version_amounts` is a full scan of a table with tens of millions of rows, and issue 243's
  lesson is that a report query gets an A/B before it is added, not after.

### Tests

`an_amount_carries_the_tax_basis_its_source_stated` pins all four cases from the parse layer through the
fold: stated exclusive, stated inclusive, not stated (NULL — the shape every existing row has), and an
undefined code (dropped). `v_tender_amounts` exposes the column, and the SQL surface's own description
of the view now warns that most rows are NULL so a total over mixed rows is not comparable.
