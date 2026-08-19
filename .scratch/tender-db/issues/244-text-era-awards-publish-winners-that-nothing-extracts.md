# 244 — 1.3M text-era award notices publish their winners in prose, and nothing extracts them

Status: SLICE 4 BUILT 2026-08-19 — the contract price, at notice scope, claimed only from one
unambiguous shape (one number, one three-letter currency code, nothing else); everything ranged,
dual-currency, tax-qualified, annualised, sub-cent or withheld is refused rather than guessed.
No projection change: `TED-VAL_TOTAL` at root is already mapped to `result_value`. Slice 3 is
DEPLOYED and A/B-verified — winners went 12 → 755 → 3,807 on one package, 0.3% → 18.2% → 91.7% of
its 4,153 award notices. AWAITING DEPLOY + the value A/B. Still open: the award date and the
tenders-received count (both need canonical destinations), and the 2010 tail.
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


## Campaign running (2026-08-19), and what it taught in its first hour

The re-parse is going at ~190 notices/s per virgin package (issue 248 explains why it can), each package
followed by its own fold. Verified on `fetch 252`, one the campaign had just done:

    297 of 700 sampled notices carry a LotResult section  (42%, the era's award share)
    628 winner parties from 300 sampled notices, with real names —
        Total France 19, Balton Spółka z o.o. 13, ETDE Ouest 12, Hurtownia Farmaceutyczna Ismed Sp. J 10

That last name is a defect the campaign's own output revealed: `Sp. J.` had lost its period, because the
abbreviation check looked at the last dot-separated segment WITHOUT trimming it and so saw ` J` rather
than `J`. `sp. j.` is an ordinary Polish legal form, and since these winners carry no identifier the name
IS their identity — `Sp. J` and `Sp. J.` would be two organizations for one company (issue 234's failure).
Fixed, with the case pinned by a test, while the campaign is 15 of 215 packages in: redoing 15 packages is
cheap, redoing 200 would not be.

Packages re-parsed before that fix, and before the `NAME_WINDOW` and non-ASCII fixes earlier today, want
redoing at the end of the campaign — the list is `fetch 186` and `240`–`255`-ish, and they are also the
slow ones (their section ids genuinely changed, so their mentions must go).


## The pre-2004 grammar, measured and implemented (2026-08-19, mid-campaign)

The campaign walks the era **backwards in time** — `fetch 251` is 2005-07, `310` is 2000-08, `330` is
1998-12, `400` is 1993-02. So it crossed the 2004 boundary around `fetch 270` and has since been
re-parsing vintages whose award notices this extractor could not read. Measured on `fetch 300`
(2001-06), a package the campaign had already completed, over a bounded 2,000-notice band:

    TD:7 award records in the band                                  619
    notices whose TX contains `HAS BEEN AWARDED`                      1
    notices whose TX contains `SERVICE PROVIDER:`                     1
    notices with a TED-ADDRESS_CONTRACTOR id-ref (extracted winner)   1

Original language in the same band: FR 675, EN 411, DE 378, IT 149, ES 146, NL 89, SV 43, DA 28 — so
English is a fifth of it, and even against ~227 English awards one winner is 0.4 %. Compare `fetch 252`
(2005-06), verified earlier in the campaign: **42 %** of notices carry a LotResult. The gap is the
vintage, not the language and not the extractor's fidelity: on the labels it targets it is at 1-for-1
in this band too.

### The 2001 grammar, read from three payloads

Pre-2004 award notices use a **numbered-item** form, not sections. Two shapes, both from `fetch 300`:

    notice 1,710,454 — works/services, winner at item 6
      1.  Awarding authority: Redcar and Cleveland Borough Council, …
      3.  Date of award: 30.3.2001.
      5.  Tenders received: 2.
      6.  Successful contractor(s): Mill Group, 3 Burlington Mews, UK-London W1R 8QA.
      7.  Works provided: CPV: 45210000, 74222000, 74873100.
      8.  Price: …            9.  Value of winning award(s): …

    notice 1,710,387 — EC external aid (SCR/EuropeAid), winner at item 8
      4.  Contract value: 2 143 000 EUR.
      5.  Date of award of the contract: 11.5.2001.
      6.  Number of tenders received: 6.
      8.  Name and address of successful tenderer: Symonds Travers Morgan Ltd (UK) in
          association with Tecnica y Proyectos SA (ES), Symonds House, …

Note what else is in there, labelled and parseable, beyond the winner: an **award date**, a **contract
value** (`Price:`, `Contract value:`, `Value of winning award(s):`) and a **tenders-received count**.
That is issue 232's "text era buyers/values/winners near zero" sitting in plain prose. This slice takes
only the winner; the value and date are the next one, and they should be taken before the era's final
re-parse rather than after.

### Landed

`AWARD_LABELS` gains `Successful contractor(s):` / `Successful contractor:` and
`Successful tenderer(s):` / `Successful tenderer:` (upper-cased, matched case-insensitively, both
spellings because the era writes both). Two supporting changes the measured bodies forced:

- **`NAME_STOPS` gains ` 7.` and ` 9.`** — the next numbered item after the winner. Without it a winner
  whose address carries no comma runs on: `Successful contractor(s): ACME Ltd. 7. Works provided: CPV:
  45210000, 74222000` would name the organization `ACME Ltd. 7. Works provided: CPV: 45210000`.
- **`NAME_REJECTS`** — the era fills a *withheld* item with boilerplate rather than leaving it blank
  (`Successful contractor(s): Publication of this information would prejudice the legitimate commercial
  interests of a particular undertaking.`; 2001-06 uses it for items 8, 9 and 10 of one notice). That
  sentence reaches a comma well inside `NAME_WINDOW`, so the runaway-value fall-through does **not**
  catch it — without the reject list it is minted as an organization, once per withholding notice, which
  is issue 234's identity-less provisional org manufactured on purpose. Falsified: with the guard
  removed the test fails with the sentence as the name.

Tests: the three prod bodies verbatim, both singular spellings, the no-comma boundary, and the withheld
case asserted to yield neither a name nor a LotResult section.

### Consequence for the campaign

Everything from `fetch 318` (2000-01) backwards now extracts on its **first** pass. The pre-2004
packages already done — roughly `fetch 270`–`317`, i.e. 2004-01 down to 2000-01 — join the redo list,
and a redo is the cheap kind (~2 min: it keeps every section id). The 2004-2005 packages already done
(251-269) are unaffected: their sectioned labels were read correctly the first time.

Still open for the era: the 1993-1997 flat grammar (`Supplier(s):`, ~50-64 % from the earlier sample),
the value/date/tender-count fields above, and the 2010 tail (171 of 244 award notices with no text
values at all).

### The labels are English even when the notice is not — so this is era-wide, not an English slice

The obvious worry about a grammar built from English labels, in an era whose bodies are 20 % English
(the 2001-06 band: FR 675, EN 411, DE 378, IT 149, ES 146, NL 89, SV 43, DA 28), is that its ceiling is
that 20 %. **Checked before recording it, and it is false.** Prod notice 1,710,458 carries `OL: FR`, a
French buyer and a French winner — under English structural labels:

    1.  Awarding authority: Communauté urbaine de Lyon, délégation générale aux services
        urbains et à la proximité, …, F-69399 Lyon Cedex 03.
    6.  Successful contractor(s): Groupement d'entreprises CGEV Rhône-Alpes/Parcs et Sports.
    7.  Works provided: CPV: 45112430, 77321000.
    8.  Price: 5 301 802,22 FRF TTC.

TED's tagged format labels the *form* in English and leaves only the *content* in the original
language. So the ceiling for this slice is the era's award notices, not its English ones. That body is
now a test: it also has no comma before its period (so the ` 7.` stop is what keeps item 7 out of the
name) and `Rhône` puts a multi-byte character inside the byte window.

### What item 8 says about the value slice

`8.  Price: 5 301 802,22 FRF TTC.` — the next slice is harder than it looks, and needs its own care:

- **decimal comma**, not a point, and the comma is also `NAME_STOPS`' first boundary;
- **space thousands separators** (`5 301 802`), which the wrap-flattening turns into ordinary spaces;
- **pre-euro currencies** (FRF here), so ADR-0010's integer cents need the currency to scale;
- **tax qualifiers** (`TTC` = incl. tax; the UK bodies write `p.a.`), which change what the number means;
- three different labels for it (`Price:`, `Contract value:`, `Value of winning award(s):`) plus
  `Total final value of the contract:` in the sectioned form.

Getting a value wrong is worse than not having it, so this wants the same measure-first discipline: read
a sample of each label's values off prod, then parse, then A/B one package.

### A/B across the deploy boundary: 0.1 % → 20.5 % of award notices

The grammar deployed mid-campaign, so four adjacent packages of the same vintage straddle the boundary —
the cleanest A/B this issue is going to get, on real data, at package scale:

    fetch  period    notices  TD:7 awards  winners  binary
      317  2000-01    12,141            —        6  old
      318  1999-12    10,294        3,450        4  old      0.1 % of awards
      319  1999-11    11,886        3,889      797  new     20.5 % of awards
      320  1999-10    13,076            —      811  new

A ~200× step, and honestly stated: **four fifths of the era's award notices are still unread.** The
numbered form is not one form, it is a family, and the winner's item number and label vary by directive.

### The two shapes that still miss, read from `fetch 319`

Sampled from TD:7 notices in `fetch 319` that carry NO winner after the new grammar:

**Utilities/supplies form — winner at item 9, and MORE THAN ONE of them** (notice 1,456,070):

    5.  Award procedure: Verhandlungsverfahren.
    6.  Tenders received: 11.
    7.  Date of award: 30. 8. 1999.
    9.  Supplier(s), contractor(s) or service provider(s): BP, Hamburg; Thelen, Mainz.

Three things here, each needed:

- the label `Supplier(s), contractor(s) or service provider(s):` — note it does NOT match the existing
  `SERVICE PROVIDER:`, because the era writes `service provider(s):` and the `(s)` breaks it;
- **two winners in one value, `;`-separated, each `Name, City`.** The extractor takes one name per label
  occurrence, so a `;` list needs the value split before `NAME_STOPS` is applied — a structural change to
  the scan loop, not another label;
- ` 10.` as a stop, since this form's winner item is followed by 10 rather than 7.

**EC service-award form with the winner further down than the read window** (notice 1,456,011): items 1
authority, 2 procedure chosen, 3 category and description — the winner is past 1,100 characters, so the
next slice must read a full body, not a prefix, before deciding its item number.

Both are the same discipline as this slice: read the payloads, name the labels, add the boundary the
shape needs, and A/B one package. Do the `;` split with a test that asserts BOTH names — a list read as
one name would mint `BP, Hamburg; Thelen` as an organization, which is the withheld-boilerplate failure
in a new costume.


## Slice 3: the supplies and utilities forms (2026-08-19)

### The second A/B, on the same package, against a recorded before-value

`fetch 300` (2001-06) was re-parsed under slice 2 as job 43 — the cleanest possible A/B, because this
issue had recorded its before-value that morning:

    fetch 300, 13,734 notices, 4,153 TD:7 award records
      before slice 2:   12 notices with an extracted winner   (0.3 % of awards)
      after  slice 2:  755 notices with an extracted winner   (18.2 % of awards)

Which matches `fetch 319`'s 20.5 % from the boundary A/B, so the figure is a property of the vintage
rather than of one package.

### Where the other four fifths were, counted rather than guessed

One scan over the package's 13,734 bodies:

    …successful contractor…                                785
    …successful tenderer…                                  122
    supplier(s):                                         1,435
    supplier(s), contractor(s) or service provider(s):      410
    contractor(s):                                          735   (mostly the two above)
    service provider…                                    2,323

`SUPPLIER(S):` alone is the biggest remaining label, and the combined heading's tail
`SERVICE PROVIDER(S):` covers the utilities form — note the *existing* `SERVICE PROVIDER:` does not
reach it, because the era writes `service provider(s):` there and the `(s)` breaks the match.

**And they are award-exclusive.** Every one of the **3,857** bodies in the package carrying any of the
three new labels is TD:7 — zero on TD:3 invitations, TD:2 corrigenda or anything else. So the new
labels cannot invent a winner on a notice that has no award, which was the risk worth checking before
deploying them. 3,857 of 4,153 award notices is 92.9 % carrying a label; the guards below then decide
how many of those are really names.

### What the payloads forced beyond the labels

**Two winners in one value** (notice 1,456,070, utilities): `9.  Supplier(s), contractor(s) or service
provider(s): BP, Hamburg; Thelen, Mainz.` The scan now works in two levels — an **item stop ends the
VALUE**, then `;` separates winners inside it and `,` ends each name. Reading that as one name would
mint `BP, Hamburg; Thelen` as an organization. Falsified: without the split the test gets `["BP"]`.

**A count where a name should be** (notice 21,123, 1993-02): `6.  Supplier(s): 99.` — under the
supplies form that item sometimes holds the *number* of suppliers. `plausible_name` now requires at
least one alphabetic character. Falsified: without it the test gets `["99"]`.

**Lot-keyed winners, which turned out to be a third of the era's oldest awards.** The committed
`1993-daily-en-19930102` fixture — 199 records, already in the suite — carries `6.  Supplier(s):` in
every one of these spellings:

    A: Apotecnia, Climo
    1: Ailsa Truck and Bus Limited, 101 Kelburn Street, …
    1/2: Evans MacShaw Leyland DAF Limited, Shefford Road, …
    1, 2: Carlier Chaines, 37/41, rue Roger Salengro, …
    1, 2, 3 and 4: Dolmen Computer Applications NV, …
    1: Baxter Healthcare; 2: B. Braun Medical; 3: Fresenius …
    1: Discol. 2: Rault. 3: Discol. … 14: Sarl Fuseau
    1. Poul Pedersen A/S

The earlier stance — fail closed on a lot list — was the wrong call: it drops the winners rather than
reading them. A leading lot reference is now stripped (digits, single letters, `/`, `,`, `and`, short),
and the last shape shows the separator is not always `;`: the supplies form ends each entry with a
**period** and opens the next with its lot reference, fourteen winners in one item.

Two rules keep that from eating real names. A lone letter before a period is an **initial**, not a lot
(`H. Meyer GmbH`, `B. Braun Medical`, `T.C. Harrison Group Limited` all survive), so a period-terminated
reference must be digits. And a prefix only strips if it is short and entirely lot-reference material,
so the fixture's two real consortium designations — `ARGE: Walter-Bau-AG` and `Groupement solidaire:
Entreprise Quille` — keep their colons.

**Measured on that fixture, as a committed gate:** the 199 records yield **117 winners**, up from 40
before lot-keyed and period-separated lists were read. The count is asserted exactly, and every name is
checked for the two ways a bad boundary shows: a lot reference left on the front, and a value long
enough to be an address.

**A cancelled procedure** (notice 1,710,467): item 6 empty, item 11 `Procédure annulée`. No winner
exists and none is invented — so 100 % of TD:7 is not the target, and never was.

### Reach

`Supplier(s):` is the same label in 1993 as in 2001 (notice 21,133: `6.  Supplier(s): CAMST Scrl, via
Tosarelli 318, …`), so this slice reaches the era's oldest packages, not just the pre-2004 middle. The
campaign has not walked back that far yet, which means those packages get it on their first pass.

### The third A/B: 0.3 % → 18.2 % → 91.7 % on one package

`fetch 300` (2001-06) has now been re-parsed three times, against a before-value this issue recorded
before any of it was written. Same package, same 13,734 notices, same 4,153 TD:7 award records, same
bounded notice-id band:

    pass                      notices with a winner   winner refs   % of the package's awards
    before slice 2                               12            12                        0.3 %
    after  slice 2 (numbered form)              755           755                       18.2 %
    after  slice 3 (supplies + utilities)     3,807         6,166                       91.7 %

**6,166 refs from 3,807 notices** — 1.6 winners per award notice, so the multi-winner values are landing
rather than collapsing to their first name.

91.7 % against the **92.9 %** of award notices that carry any of the labels at all: the extractor now
reads 98.7 % of the notices where a label exists. The rest is the guards firing as designed — a count
where a name should be, a withheld value, `Various.` — plus whatever shape is still unnamed. That is
close enough to the ceiling that the next work on this issue should be the **value/date/tenders-received
fields**, not more winner labels.


## Slice 4: the price, at notice scope, in one shape only (2026-08-19)

Winners are at 91.7 % of the vintage's award notices, so the next fact is the money — issue 232's
`value` column, and the one this issue has been pointing at since the 2001 payload read.

### Where it is, counted

Over `fetch 300`'s 13,734 bodies (4,153 TD:7 award records):

    Price:                        3,081        Date of award                 2,991
    Value of winning award…         483        Tenders received:             2,787
    Contract value:                  25        Estimated value                 467

So the money is stated about as often as the winner is.

### Destination: no projection change needed

`("TED-VAL_TOTAL", "result_value")` is already in the projection's `AMOUNTS` map, so a
`TED-VAL_TOTAL` amount at notice scope reaches `tender_version_amounts` as `result_value` — the same
fact the r209 era publishes under the same field id, arrived at from prose instead of a tag.

Notice scope, deliberately, **not** the LotResult: the value is stated once per notice (`8. Price:`)
while a notice can name several winners, so attaching it to the first result would attribute a whole
contract to one of them.

### One shape, and a long list of refusals

A wrong amount is worse than a missing one — it lands in a fact table and nothing downstream can tell
it from a published figure. So: exactly one number and exactly one three-letter upper-case currency
code, in either order, and nothing else in the item but a closing period.

    2 143 000 EUR.                                    claimed
    EUR 1 131 079,99                                  claimed
    5 301 802,22 FRF TTC.                             refused — a tax basis this column lacks
    562 680 GBP p.a.                                  refused — annual, not a total
    15 564 000 ATS / 1 131 079,99 EUR.                refused — two currencies
    Minimum/maximum: Lit 2 610/Lit 3 289.             refused — a range
    Lit 1 000 000 000.                                refused — `Lit` is not a code
    1 000,255 EUR                                     refused — sub-cent (ADR-0010)
    2 14 3000 EUR                                     refused — groups are not thousands
    Publication of this information would prejudice…  refused — withheld

And two labels in one notice **disagreeing** (`8. Price:` 1 000 000 EUR beside `9. Value of winning
award(s):` 900 000 EUR) claims nothing rather than guessing which figure the analyst wanted; agreeing,
it is one fact stated twice and is read.

Everything refused stays exactly where it already was — inside the `TXT-TX` prose claimed as a whole —
so a refusal costs a fact and never exhaustiveness (ADR-0004).

### The claim is gated on the notice being an award — and that gate was measured, not assumed

Winner labels needed no such gate: all 3,857 bodies carrying one are `TD:7`. The price labels are
**not** that clean. Of the 3,250 bodies in `fetch 300` stating one:

    TD:7  awards                      3,227
    TD:3  invitations to tender          18
    TD:0                                  4
    TD:2  corrigendum                     1

A notice with no result must not carry a `result_value`, so those 23 are exactly the wrong facts to
refuse — and refusing them needs the document type, which is why the claim is a **post-pass over the
finished record** rather than a hook in the prose flush: `TD` may be consumed before or after `TX`
depending on field order, and a post-pass sees both. A record with no `TD` at all is not assumed to be
an award either.

### One mechanical thing the boundary forced

The value item is 4, 8 or 9 depending on the form, so the item that follows it is 5, 9 or 10 —
`ITEM_STOPS`, aimed at the winner item, does not bound it. `next_item_marker` finds any ` <n>. `
instead, and has to tell an item marker from a date (`11.5.2001` — no space after `11.`) and from a
house number (`Emilienstrasse 8,` — no period).

### Gate

The committed 1993 daily asserts **zero** prices, which is the correct answer for that vintage: it
writes the lira as `Lit 1 000 000 000` and its ranges as `Lit 2 610/Lit 3 289`, and neither qualifies.
Asserting the zero is how a future loosening of `parse_money` announces itself.


### The value baseline, read before the deploy

`notice_amounts` holds **zero rows** for `fetch 300`'s 13,734 notices — the text era has never had a
single amount in the parse layer, so this A/B starts from a clean zero and any number after the deploy
is pure gain. The denominator to judge it against is the **3,227** `TD:7` bodies of that package that
state a price label; the strict shape will claim some fraction of those and the rest are the refusals
above, which is exactly the number worth knowing.
