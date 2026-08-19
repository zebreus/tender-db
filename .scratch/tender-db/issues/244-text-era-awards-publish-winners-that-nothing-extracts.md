# 244 — 1.3M text-era award notices publish their winners in prose, and nothing extracts them

Status: SLICE 1 DONE and VERIFIED ON PROD 2026-08-19 (2 of 215 packages re-parsed). The remaining 213
packages are a staged campaign; pre-2004 grammar, values and the 2010 tail still open.
Kind: extraction gap, the largest single cohort in the corpus
Blocked by: — (wants to ride along with the text-era re-parse already planned for the AU→buyer fix)
Relates to: 235 (the denominator that made this visible), 242 (the column it landed in), 13 (results
layer), 100 (winners unresolved, the eForms half of the same question), 41/202 (text-era ingestion),
the pending text-era re-parse for the `AU:`→buyer mapping

## What

Section 3 of the first full-corpus run, text era:

    era                            award-notices with lot_results  density  no block parsed
    text 1993–2010                  1,306,514              0     0.0%        1,306,514

**1,306,514 award notices, zero materialised results.** By count that is the largest cohort in the
report — bigger than r2.0.8 (1,088,292) and r2.0.9 (2,098,599 award notices at 100.0 %) put together
in terms of what is missing.

And the content is published. From the committed 2005 fixture
(`tests/fixtures/text/2005-can-154-2005.txt`, `TD: 7 - Contract award`):

    CO: Name and address of successful supplier, contractor or service provider:
        Grahams Engineering Ltd.
        NSG Environmental Ltd.

Two named winners, in the era's ordinary labelled-line format with continuation lines — the same shape
every other text-era field uses. The parser maps `CO` as `Prose(None)` (`text/rules.rs`), so the text
is claimed and retrievable as `TXT-CO` under ADR-0004, and then nothing turns it into a result block, a
`lot_results` row, or a winner mention.

## Why it was invisible until now

The old section 3 drew its denominator from notices that already carried a parsed result SECTION, so
an era that parses no result sections at all fell out of both halves and simply did not appear. Issue
235 moved the denominator onto the notice's own published document type (`TXT-TD = 7` here), and the
era appeared — with the largest gap in the report. This is the exact failure class issue 235 was
opened to expose, at a scale nobody had guessed.

## Measured on prod, 2026-08-19 — the content is there, and `CO:` is not the way in

Bounded per-year probes (800 text-era notices per window, keyed lookups, ~5 ms each), counting award
notices by their own published type (`TXT-TD = 7`) and asking which carry a `TXT-CO` field:

| year | sampled | awards | with `TXT-CO` |
|------|---------|--------|----------------|
| 1993 |     800 |    281 |    0    (0 %)  |
| 1994 |     800 |    294 |  276  (94 %)   |
| 1995 |     800 |    217 |  194  (89 %)   |
| 2000 |     800 |    320 |  306  (96 %)   |
| 2003 |     800 |    281 |  279  (99 %)   |
| 2006 |     800 |    354 |  343  (97 %)   |
| 2009 |     800 |    506 |  501  (99 %)   |

**But `TXT-CO` turned out to be a heading, not a value.** On the 2006 rows it holds exactly
`NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN AWARDED` and nothing else — one
ordinal, no continuation. The committed 2005 fixture, where `CO:` is followed by two supplier names on
continuation lines, is **not** representative of the era; it is one of at least two shapes.

Where the award content actually lives is the prose body, `TXT-TX`, and it is structured. Notice
2,821,477 (2006-06, F-Nice demolition works), body 1,081 chars, at offset 773:

    SECTION V: AWARD OF CONTRACT
    V.3)  NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT HAS BEEN
    AWARDED: Gagneraud Construction, 198 chemin des Eucalyptus, F-06160
    Antibes-Juan-les-Pins.
    V.4)  INFORMATION ON VALUE OF CONTRACT Total final value of the …

Winner **and** value, under TED's own numbered form headings. And the 1993 rows — the ones with no
`CO` at all — publish the same facts in a different, equally labelled grammar (notice 17,429):

     1.  Awarding authority: Clydesdale District Council, …
     2. (a)  Award procedure: Open procedure.
     3.  Date of award: 1: 1. 12. 1992. …
     6.  Supplier(s): A: Apotecnia, Climo, Fo…

So the era publishes its awards throughout, in at least two machine-readable grammars:

- **flat numbered list** (1993-era): ` N.  Label: value`, winner under `Supplier(s):`, date under
  `Date of award:`.
- **numbered form sections** (by 2006, presumably arriving with the 2002/2004 directive forms):
  `SECTION V: AWARD OF CONTRACT` then `V.3) …AWARDED: <name>`, `V.4) INFORMATION ON VALUE …`.

`TXT-CO` is still useful as a **cohort marker** for the second shape (it is the section heading, so its
presence says "this notice has an award section"), just not as the value carrier.

## What is still unknown

1. **Where the grammar switches.** `CO` appears in 1994 and the `SECTION V` form is in place by 2006;
   the boundary years are unprobed. A per-year probe for `SECTION V` vs `Supplier(s):` inside `TXT-TX`
   maps it exactly — the same bounded shape as the table above.
2. **Language.** `TXT-TX` carried the English text in both samples and `TXT-OT` the original language
   (French for the Nice notice). Whether English `TXT-TX` is universal across the era, or whether some
   notices only ever got the original language, decides whether the labels can be matched in one
   language or several.
3. **Value parsing.** `V.4) INFORMATION ON VALUE OF CONTRACT` was truncated in the sample; the currency
   and amount format needs its own look before any amount is claimed.
4. **Multi-award notices.** The 1993 example lists three award dates (`1:`, `2:`, `3:`) and several
   suppliers, so one notice can carry several awards — the target shape must be per-award, not one
   winner per notice.

## Steps

1. Probe the grammar boundary (step 1 above) and the language question (step 2) — bounded, cheap, and
   they decide the parser's shape.
2. Extract in the parse layer, not the fold: the era's payloads are stored, so a re-parse can promote
   the award facts out of `TXT-TX` into a proper result section + winner mention, which every existing
   projection then folds for free.
3. Fixtures from at least three points of the span (1993 flat list, an early-2000s notice, a 2006+
   `SECTION V`) before writing the mapping.
4. Land it in the same re-parse pass as the `AU:`→buyer fix — both need every text-era notice re-read,
   and doing that twice for one era is the expensive way.
5. Re-run the report: the text era's density in section 3 is the acceptance number.

## Note on the report's own wording

The first render of this column said "Nothing can project those" for the whole of it, which was true
for r2.0.8's 14,532 and false for these 1.3M. Corrected in the same commit that files this issue: the
column measures the PARSE, and the two causes — publisher shipped nothing, or we do not extract it yet
— now get named separately. A correct number under a confident wrong sentence is worse than no
sentence.


---

## The remaining open questions, answered on prod (2026-08-19)

**1. Where the grammar switches: between 2002 and 2004.** Per-year June windows, award notices by their
own `TXT-TD = 7`, asking what their English body contains:

| year | awards | `SECTION V` | ` Supplier(s):` |
|------|--------|-------------|------------------|
| 1994 |    158 |   0 |  75 |
| 1997 |    207 |   0 | 133 |
| 2000 |    167 |   0 |  80 |
| 2002 |    190 |   0 |  97 |
| 2004 |    203 | 200 |   0 |
| 2006 |    354 | 349 | 347 |
| 2009 |    314 | 312 | 312 |

Clean break at the 2004 directive forms. Note the early grammar's `Supplier(s):` reaches only ~50–64 %
of its awards, so the pre-2004 slice needs more label discovery before it can be written — the other
half labels the winner some other way.

**2. Two label vintages inside the SECTION-V era**, not one:

| year | `AWARDED:` | `PROVIDER:` | either |
|------|-----------|-------------|--------|
| 2004 |   0 | 202 | 202 / 203 |
| 2005 |   0 | 311 | 311 / 314 |
| 2006 | 347 |   0 | 347 / 354 |
| 2008 | 361 |   0 | 361 / 371 |
| 2010 |   0 |   0 |   0 / 244 |

2004–2005 use `Name and address of successful supplier, contractor or service provider:` (the same
wording as the era's `CO:` line); 2006+ use `NAME AND ADDRESS OF ECONOMIC OPERATOR TO WHOM THE CONTRACT
HAS BEEN AWARDED:`. Both end at a colon the value follows, and both wrap mid-heading, so the match has
to be whitespace-normalised and case-insensitive.

**3. Language, and a new anomaly in the era's tail.** In the 2010 window only 73 of 244 award notices
have ANY `notice_texts` row at all — no `TXT-TX`, no `TXT-AU`, nothing — while all 244 carry the codes
that identify them as awards. So the 2010 slice is not a language problem but a content one, and it is
unexplained. Filed as its own question below rather than guessed at.

## Slice 1 landed (2026-08-19) — the 2004+ grammar

`awarded_names` in the text parser matches both label tails on the whitespace-flattened body and takes
the name up to the first comma; `Emit::award` manufactures, per occurrence, a `LotResult` section, an
`Organization` inside it, and a `TED-ADDRESS_CONTRACTOR` id-ref between them — the shape
`read_legacy_results` and `legacy_role` already read for every legacy profile. No projection change was
needed: the era needed a section to find, not new folding code.

Two details worth keeping in mind for the next slice:

- The name is filed as **`TED-OFFICIALNAME`**, because `ORG_NAME_FIELDS` reads only that and `TXT-AU`.
  Filing it under the era's own `TXT-CO` — the first attempt — produced a nameless organization, and no
  parse-layer test would have caught it. The integration test now asserts through to
  `tender_version_parties`.
- **Only the name, never the address.** One prod notice awards four contracts to "Stryker France" at
  "Zac Satolas Green" and "Zac de Satolas Green"; an address-bearing name mints an organization per
  spelling, which is issue 234's problem made worse on purpose.

Multi-award notices are ordinary: 25 % of a 2008 window awards more than one contract, up to 18, each
under its own `CONTRACT NO:`.

## Still open

1. **The re-parse.** Nothing changes for the stored corpus until the era is re-read from the archive.
   That pass also carries issue 232's `AU:`→buyer fix, which is why both waited for one pass.
2. **Pre-2004 grammar** (~half the era's award notices by count): `Supplier(s):` covers only ~50–64 %,
   so the first step is finding what the rest use.
3. **Values.** `V.4) INFORMATION ON VALUE OF CONTRACT` publishes `Value: 303 504,79 EUR.` — space
   thousands, comma decimal, sometimes VAT lines after it. Not attempted; a wrong amount is worse than
   no amount.
4. **The 2010 tail**: 171 of 244 award notices in the sampled window have no text values at all. Needs
   its own investigation — possibly related to issues 139 (the 2010-03 DTD population) or 199.


---

## Slice 1 verified on prod (2026-08-19), and what the staging found

Two packages re-parsed with the new `packages` / `after` caps, each followed by its fold.

**`fetch 186` — 35,830 notices, 1,012 members, 0 unmatched, 0 now failing — and zero result sections.**
Correct, and the reason is the point: that package is 2010-12-01..03, and the 2010 tail is exactly the
slice with neither award label. Had the run not been capped, this would have looked like the extractor
silently doing nothing across the whole era.

**`fetch 240` — 20,755 notices (2006-06) — the extractor working:**

    sampled 800 notices of the package
    with a LotResult section          346
    with a TED-OFFICIALNAME winner    346

and through the fold into the canonical layer:

    winner parties from 300 sampled notices   605
    Polatom Sp. z o.o 12, TBS-FR 8, Sodiprho 7, PGF Urtica Sp. z o.o 7,
    STMI 6, Farmacol SA 6, Techniques et technologies 5, Gambro Poland Sp. z o.o 5

Real company names, several winners per award notice (the multi-contract shape), reaching
`tender_version_parties` with role `winner` — which is the whole chain the era has never had.

Two cosmetic notes for whoever reads these names later: the trailing period of an abbreviation is
trimmed (`Sp. z o.o.` → `Sp. z o.o`), deterministically, so mentions still merge; and the name is
whatever precedes the first comma, so a company whose legal name contains a comma will be cut at it.

`0 now failing` on both packages is the line that matters for a parser change: no notice that parsed
before stopped parsing.

## The remaining campaign

213 packages left, at roughly 4–5 minutes each plus a fold — call it 15–20 hours of queue time, so it is
a staged campaign across firings, not one job. `after` + `packages` make each step selectable and
resumable, and the summary names what the cap held back.

One thing checked and found harmless: each re-parse reports `stamped 2,601,443 tender(s) epoch-stale`,
the same number both times, so the stamp is the whole legacy cohort and re-stamping is idempotent — it
is not accumulating per-package debt. The fold each time touched only the re-parsed notices' own tenders
(19,335 and 33,160).
