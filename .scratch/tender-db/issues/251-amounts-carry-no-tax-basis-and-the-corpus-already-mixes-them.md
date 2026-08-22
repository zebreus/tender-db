# 251 — amounts carry no tax basis, and the corpus already mixes inclusive with exclusive figures

Status: CLOSED 2026-08-22 — all four steps done; r208 verified (0.7 % → 13.5 % stated), r209 acquitted by probe (the source rarely states a basis). Was: TEXT ERA DONE AND VERIFIED ON PROD (real rows: 504 excl / 178 incl / 73 NULL over notices
3,870,856-3,875,000); FORM ERAS DEPLOYED 2026-08-20 as `5a6e676`; OPTION 2 (the report line) BUILT
2026-08-20 — section 6 of the data-quality report, three-way with the bias caveat. What remains is the
r208/r209 RE-PARSE (not a refold — `INCLUDING_VAT` is a parse-layer change) that makes the form eras'
mix real, and the first report run that reads section 6
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


---

## VERIFIED ON PROD (2026-08-20, rev `96d63a2`)

`fetch 300` re-parsed and folded under the deployed rev; four notices whose prose I had already read,
followed all the way into `tender_version_amounts`:

    notice      field         cents        currency  tax_basis   the prose it came from
    1,710,387   result_value  214,300,000  EUR       NULL        "4.  Contract value: 2 143 000 EUR."
    1,710,441   result_value   68,965,517  DEM       excl        "8.  Price: Auftragssumme (ohne
    1,710,442   result_value  194,425,500  DEM       excl         Umsatzsteuer): 689 655,17 DEM." etc.
    1,710,443   result_value   12,066,233  DEM       excl

Every part of the chain holds: the figure parses to the right cents (`689 655,17` → 68,965,517), the
German sub-label's `ohne Umsatzsteuer` becomes `excl`, and the external-aid notice that states no basis
correctly gets **NULL** rather than a guess.

So the corpus now has its first labelled money, and the labelling came from what the publisher actually
wrote rather than from an assumption about what era usually means.

## Follow-ups, unchanged

1. **r208/r209 pairing** — where most of the corpus's money is. `EXCLUDING_VAT` is a presence flag and
   `INCLUDING_VAT` a container, so neither is a code carrying `incl`/`excl`; pairing needs a payload read
   of how they sit relative to the value element. The mechanism (`TAX_BASIS_FIELDS` plus the
   section-keyed map) is in place for them to join.
2. **The report line** (option 2) — a `GROUP BY tax_basis` over tens of millions of rows, so it gets an
   A/B before it is added. The 2026-08-20 cost table in issue 253 is the baseline to A/B against.

---

## The r208/r209 shape, read from committed fixtures (2026-08-20)

The follow-up above said pairing the form eras "needs a payload read of how those elements sit relative
to the value element". Done, from three committed fixtures — no prod access needed. It changed the
design twice, so the reading is worth recording in full.

### The shape

    <COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE CURRENCY="EUR">
      <VALUE_COST FMTVAL="592140">592 140</VALUE_COST>
      <EXCLUDING_VAT/>
    </COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE>

`EXCLUDING_VAT` is an empty **sibling** of the value, inside a container that groups the two.

### First reading, and why it was wrong

One section can hold **several** amounts with different bases. From the defence award
(`f18-defence-001420-2019`), inside a single `AWARD_OF_CONTRACT_DEFENCE` section:

    <CONTRACT_VALUE_INFORMATION>
      <INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT CURRENCY="RON">
        <VALUE_COST FMTVAL="2162630.19">2 162 630,19</VALUE_COST>      ← no marker here
      </INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT>
      <COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE CURRENCY="RON">
        <VALUE_COST FMTVAL="1681100">1 681 100</VALUE_COST>
        <EXCLUDING_VAT/>
      </COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE>
    </CONTRACT_VALUE_INFORMATION>

So the section-keyed map the text era uses would have labelled the **initial estimate** `excl` on the
strength of a marker belonging to the **final value**. That is a wrong fact, of exactly the class this
whole slice exists to avoid, and it would have been invisible.

### Second reading: the prefix already IS the container

The r208 fixture (`f03-annexd-neg-022211-2011`) shows `EXCLUDING_VAT` inside
`INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` **as well**:

    …<VALUE_COST FMTVAL="375000.00">375 000</VALUE_COST><EXCLUDING_VAT></EXCLUDING_VAT>
    </INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT>

And `INITIAL_ESTIMATED_TOTAL_VALUE_CONTRACT` is in `FIELD_PREFIX_WRAPPERS`, while
`COSTS_RANGE_AND_CURRENCY_WITH_VAT_RATE` is not. So the parse layer already keeps the pairs apart by
field id:

    TED-VALUE_COST                                        ↔  TED-EXCLUDING_VAT
    TED-INITIAL_…_VALUE_CONTRACT.VALUE_COST               ↔  TED-INITIAL_…_VALUE_CONTRACT.EXCLUDING_VAT

**The prefix is the container identity.** The rule is therefore exact rather than heuristic: for an
amount whose field id ends in `VALUE_COST`, the basis is the marker in the same section whose field id
is that id with the trailing element swapped for `EXCLUDING_VAT`. No uniqueness guard needed for the
prefixed cases; the only residual ambiguity is two *unprefixed* `COSTS_RANGE` containers in one section,
which none of the three fixtures shows and which a "claim nothing when there are two" guard covers for
free.

### One half is invisible and needs a parser change first

`EXCLUDING_VAT` is `Rule::Marker`, so it emits `Integer(1)` and is readable. **`INCLUDING_VAT` is
`Rule::Group`** — a Group emits nothing of its own and only recurses, so an inclusive-of-tax value
currently leaves *no trace at all* in the parse layer; only its `VAT_PRCT` child survives. So:

- the `excl` half is implementable now, purely in the projection;
- the `incl` half needs `INCLUDING_VAT` moved from `Rule::Group` to `Rule::Marker` in the r209 rules —
  a parse-layer change across 7.2M notices, which wants its own unit, its own exhaustiveness check
  (a Group that becomes a Marker still has to consume its children), and an era refold to take effect.

Doing only the `excl` half would populate the column with `excl` and NULL, where some of the NULLs are
really `incl`. That is not a wrong label — NULL means "the source did not say" and this parse layer
genuinely does not say — but it is a **biased** NULL, and anyone summing by basis should know it. Both
halves should land together, or the first must ship with that caveat written into the column's comment.

### Next unit, concretely

1. Move `INCLUDING_VAT` to `Rule::Marker`; confirm its `VAT_PRCT` child still parses and nothing becomes
   unclaimed. Test on a fixture that carries it.
2. Derive the marker id from the amount id in the projection; pair within the section; claim nothing
   when a section holds two unprefixed amounts.
3. Tests over all three committed fixtures, asserting the *initial estimate* stays NULL while the final
   value gets its basis — the case the first design got wrong.
4. Refold r208/r209 and read the split.


---

## The form-era half BUILT (2026-08-20) — steps 1-3 of the four

**Step 1, the parser.** `INCLUDING_VAT` moved from `Rule::Group` to `Rule::Marker` in the r209 rules.
The two arms are identical but for one line — a Group does `no_stray_text` then recurses, a Marker does
the same plus emits `Integer(1)` — so the promotion is **purely additive**: same child handling, same
exhaustiveness, one new value. An inclusive-of-tax figure now leaves a trace where before only its
`VAT_PRCT` child survived.

**Step 2, the pairing, derived rather than heuristic.** For an amount whose field id ends in
`VALUE_COST`, the marker is that id with the trailing element swapped:

    TED-VALUE_COST                      →  TED-EXCLUDING_VAT / TED-INCLUDING_VAT
    TED-INITIAL_…_CONTRACT.VALUE_COST   →  TED-INITIAL_…_CONTRACT.EXCLUDING_VAT / …

Both bases marked at once yields nothing, and so does a section holding **two** amounts under the same
id — the one residual ambiguity the prefix cannot resolve, since two unprefixed `COSTS_RANGE` containers
would both emit `TED-VALUE_COST`. Neither is labelled rather than one being guessed.

**Step 3, the tests, and a correction to how I first wrote them.** My first test used the committed
defence award and asserted "exactly one amount states a basis" — which passes, but **not for the reason
I claimed**. Printing the table showed that fixture projects exactly ONE amount at all (the initial
estimate is unprojected by issue 177's rule), so a section-keyed lookup would have passed it too. It
proves the marker is read end to end from a real payload, and nothing about container-exactness.

The discriminating case had to be synthesised, because no committed fixture projects two amounts in one
section:

- one section, `TED-VALUE_COST` 500,000 + `TED-VAL_TOTAL` 600,000 + one `TED-EXCLUDING_VAT`: the first
  takes `excl`, the second must stay NULL. **Falsified** — pairing by section instead gives the
  `VAL_TOTAL` amount `excl` too, and the test fails with exactly that.
- one section, two `TED-VALUE_COST` amounts + one marker: neither is labelled.

The fixture test now says in the assertion what it does and does not prove, and carries a guard so that
if that notice ever starts projecting its second amount, the test asks to be revisited rather than
quietly continuing to prove less than it appears to.

**Step 4 IN FLIGHT (2026-08-21):** the marker fix is a PARSER change (`Rule::Marker`), so this is a
re-parse, not a refold. Job 302 (`reparse ted-export-r208`, 161 packages) ran clean: 2,699,213
notices re-parsed, 0 unmatched, 0 now failing, 1,455,097 tenders stamped stale.

The fold plan then CHANGED, for a measured reason: r208's delta pulls a legacy closure of
2,935,319 notices — over the 500k cap — so its paired fold correctly fell back to a WHOLE-CORPUS
re-projection (~3h, issue 58 v2's designed distrust path). Since any era-scale reparse ends in the
same full walk, and section 6 reports PER ERA (so r208/r209 attribution is independent — the
earlier one-era-at-a-time note was about attribution and is moot for two eras sharing one parser
change), fold 304 was cancelled mid-plan-build and `reparse ted-export-r209` enqueued as 305 with
its paired fold 306: ONE whole-corpus fold now serves both eras, saving a full ~3h walk. The read
is section 6 after 306 lands (or bounded per-era `/v1/sql` counts).

**Step 4 READ (2026-08-22, DQ job 307, post-refold):** a split verdict.

- **r2.0.8 VERIFIED**: stated 0.7 % → **13.5 %** (453,067 excl / 168,461 incl of 4,596,878
  amounts). Both markers flow, and the incl/excl mix (≈2.7:1) is now a procurement fact rather
  than parser history — the section-6 caveat no longer applies to this era.
- **r2.0.9 ACQUITTED (same day)**: 2,029 excl / 726 incl of 11,503,399 (≈0.02 %) — and the probe
  says that IS the source: grepping the first 300 MB of two r209 monthly packages (2018-06,
  2020-03; tens of thousands of notices each) finds `EXCLUDING_VAT|INCLUDING_VAT` just 21 and 9
  times. The r2.0.9 grammar dropped the per-value VAT indicator (it kept `VAT_PRCT` rates in
  places, which are a different fact). The mapping is right; the near-zero is publication
  reality. With that, THIS ISSUE IS DONE: every era that publishes a basis has it mapped, the
  column is measured weekly (section 6), and the caveat line stays for the eras whose mix is
  parser history.



## Option 2 — the report line (built 2026-08-20)

Section 6 of the data-quality report, `amount_basis`, per era:

    == 6. Amount VAT basis (share of projected amounts stating one) ==
      era                                amounts       excl       incl   unstated   stated

Four measured columns and one derived: `unstated` is `amounts - excl - incl` rather than a fourth
`SUM(CASE WHEN tax_basis IS NULL ...)`, so the text and the JSON cannot print a third answer that
disagrees with the three it came from. Windowed on `v.tender_id` like every other section-1 query, so
it costs one more statement per window and the existing windowed-equals-unwindowed equivalence test
covers it without being told to.

Three things it is built to say, and one it deliberately refuses to say:

- **Whether an era's column is populated at all.** The r208/r209 re-parse this issue still owes has no
  other read: `tender_version_amounts.tax_basis` non-NULL for those profiles is the acceptance test,
  and before this there was no place to see it.
- **That an era states no basis anywhere** — an era with 400 amounts and 0 stated renders as `0.0%`
  rather than dropping out of the table. Tested, because a missing row reads as "no data" and a zero
  reads as "no basis", and those are different findings.
- **That a MIX is not yet a fact.** The section prints the caveat next to the numbers: the form eras
  recorded `EXCLUDING_VAT` before `INCLUDING_VAT` was mapped at all, so an excl-heavy split there is
  parser history. Printing a ratio without that line would have manufactured a procurement finding out
  of a deployment order.
- **What the amounts MEAN together.** It does not sum or average cents across bases, because that is
  the meaningless operation this whole issue exists to prevent.

Counts amount ROWS across every version, the same way section 1 counts every version rather than only
the current one. An amount row belongs to exactly one version, which is what makes the windowed sums
exact.

Noticed while wiring it, not fixed here: `render_json` omits section 5 (`fresh_holds`) entirely, so the
JSON surface has been a section short since issue 246. Section 6 IS in the JSON (`amount_vat_basis`).
Worth a small follow-up rather than a silent asymmetry.
