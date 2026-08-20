# 244 — 1.3M text-era award notices publish their winners in prose, and nothing extracts them

Status: ERA ACCEPTANCE READ 2026-08-20 — section 3 shows the text era at **462,772 of 1,306,514 award
notices materialised (35.4 %), from 0**, with the fold's own shortfall at 0. Slices 2-6 are deployed;
the gap to the ~90 % the per-package A/B demonstrates is the redo sweep (task 38), which is now the
only remaining stage. Still open beyond it: the award date, the tenders-received count, and the 2010
tail.
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

### The value A/B: 0 → 390, and what the other 88 % refused on

Deployed as `44ac476`, `fetch 300` re-parsed, same bounded band:

    notice_amounts rows, fetch 300      before: 0        after: 390 (390 distinct notices)

Ten currencies, every one a plausible code for 2001, and the magnitudes check out — ITL from 157 M lira
(≈ €81 k) to 214 bn lira (≈ €110 M), ESP up to 8.96 bn pesetas (≈ €54 M), DEM up to 230 M, SEK up to
809 M:

    EUR 106 · GBP 60 · SEK 56 · DEM 39 · ESP 37 · ITL 28 · FIM 25 · FRF 10 · NLG 9 · PTE 7

Against the 3,227 bodies stating a price label, 390 is **12.1 %**. Sampling the refusals — the whole
point of building the strict shape first — shows they are not spread over a dozen causes. Six of eight
consecutive samples are ONE shape:

    Price: Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.
    Price: Auftragssumme (mit Umsatzsteuer): 110 761,16 DEM.
    Price: Auftragssumme (ohne Umsatzsteuer): 1 944 255 DEM.

A **local-language sub-label ending in a colon**, and then an otherwise perfect `<number> <CUR>`. The
other two samples are refused for good: `Price of product plus price of transport.` is prose, and
`Importo netto di 26 985 463 468 ITL (13 936 828,783 EUR), a cui si sommano 1 351 6…` is a net figure
with a parenthetical conversion at three decimals plus additions.

So slice 5 is small and well-aimed: **skip a trailing sub-label** — if what follows the price label
carries a colon before the number, restart after the last one — leaving the number/currency test exactly
as strict as it is.

It also forces a decision this slice ducked, so it is filed separately rather than settled here: those
sub-labels state their **tax basis** (`ohne`/`mit Umsatzsteuer`, `TTC`, `HT`) and `tender_version_amounts`
has nowhere to put it. Refusing every value that states its basis discards most of the era's money;
claiming them mixes bases in one column — which the corpus already does, since the r208/r209 eras'
published VAT indicator is not modelled either. See issue 251.


## Slice 5: skip the sub-label, read the tax marker, guard the second figure (2026-08-19)

Three rules, each from the refusal sample, and one of them reverses a slice-4 decision on purpose.

**Skip a sub-label.** A value that does not parse whole is retried after its LAST colon —
`Auftragssumme (ohne Umsatzsteuer): 689 655,17 DEM.` becomes `689 655,17 DEM`. That was six of eight
sampled refusals.

**But only when the skipped part carries no digit.** This is the whole safety of the rule and it is
worth stating plainly, because without it the shape

    1 000 000 EUR, of which subcontracted: 200 000 EUR

claims the **subcontracted** figure as the contract price. A pure label has no digits; a second figure
does. Falsified: with the guard removed the test gets `Some((20000000, "EUR"))`.

**Read the tax marker instead of choking on it.** `TTC` is three upper-case letters, so slice 4's
currency test saw it as a second currency and refused `5 301 802,22 FRF TTC` — the French shape, whole
and unambiguous. Markers (`TTC`/`TVAC` incl., `HT`/`HTVA` excl.) are now tested *before* currency codes,
and the German sub-label wording (`ohne`/`mit Umsatzsteuer`, `netto`/`brutto`) is read from the label
that was skipped. A value stating both bases states neither.

### The reversal, stated as a reversal

Slice 4 refused every value that stated its tax basis, on the reasoning that mixing bases silently into
a column that records none would be a subtle wrong. **The measurement reversed it**: that refusal drops
most of the era's money, and the column already mixes bases corpus-wide because the r208/r209 eras'
published VAT indicator is not mapped either. Refusing was not the neutral choice, it was a large
silent loss chosen to avoid a smaller inaccuracy that exists everywhere else.

So the figure is claimed **and** the basis captured, as its own parse-layer code
`TED-VAL_TOTAL_TAX_BASIS` (`incl`/`excl`) beside the amount. It has no canonical destination yet — that
is issue 251 — but recording it now means the era will not have to be re-parsed to learn what it
already said, and a parse-layer reader can already tell the two apart. The text era therefore becomes
the *best*-labelled money in the corpus rather than another unlabelled contributor.

### Unchanged

`Price of product plus price of transport.` (prose), `Minimum/maximum: Lit 2 610/Lit 3 289` (a range
whose label carries digits anyway), `15 564 000 ATS / 1 131 079,99 EUR` (two currencies) and the 1993
`Lit`-prefixed figures are all still refused. The committed 1993 daily still asserts **zero** prices.

### The fourth A/B: 390 → 984, and the refusal set is now mostly CORRECT refusals

Same package, same band, slice 5 deployed as `ac17e0c`:

    notice_amounts rows, fetch 300     0 → 390 (slice 4) → 984 (slice 5)
    tax basis captured                 269 excl · 118 incl   (387 of the 984)

984 of the 3,227 price-label bodies is **30.5 %**, up from 12.1 % — two and a half times, and the first
tax-basis data anywhere in the corpus. Below the prediction, and the reason is worth recording: the
German sub-label shape was six of *eight* sampled refusals, and eight was too small a sample to speak
for 2,800 bodies.

Sampled the refusals again. They have changed character — most are now refused **correctly**:

    Price: 1. 400 000 FIM; 2. 25 500 000 FIM.                     per-LOT price list
    Price: 1) 460 000 FIM, 2) 6 300 000 FIM.                      per-lot, other spelling
    Price: Montants des marchés TTC: sites A: 1) 262 966,85 FRF;…  per-site AND per-lot
    Price: 562 680 GBP p.a.                                        annualised, not a total
    Price: 198 741 680 ITL (102 641,511 EUR), al netto degli…      figure plus conversion
    Price: Importo netto di 26 985 463 468 ITL (13 936 828,783 EUR)…  same
    Price: Publication of this information would prejudice…         withheld
    Price: Price of product plus price of transport.                prose

The dominant remaining class is a **per-lot price list**, and refusing it at notice scope is the right
answer, not a gap: there is no single contract price to record, and claiming one figure — or a sum —
would invent a fact. Reading those properly means attributing a value per LotResult, which is a
different unit from this one and needs the lot keys the era does not publish in the award block.

One easy recovery did fall out of the sample and is included here: `8 600 000 DEM netto.` — the German
pair also appears as a bare word AFTER the figure, with no sub-label and therefore no colon to retry
past, so it was an unknown token that refused the whole value. `netto`/`brutto` are now markers like
`TTC`/`HT`.

So the honest position for the era's money at notice scope: **near its ceiling**. What is left is either
per-lot (structural), a converted or annualised figure (a different fact), or withheld.


### The fifth A/B: 996, and the price arc closed

`netto`/`brutto` as bare trailing words, deployed as `2f48c0c`:

    notice_amounts rows, fetch 300     0 → 390 → 984 → 996
    tax basis captured                 279 excl · 120 incl   (399 of the 996)

+12 rows, which is the right size for a narrow shape and confirms the sample was read correctly. The
arc for this package's money is therefore **0 % → 30.9 %** of its 3,227 price-label bodies, and the rest
is the refusal set analysed above — per-lot lists, conversions, annualised figures, withheld, prose —
which is where it should stay until a value can be attributed per LotResult.


## THE ERA ACCEPTANCE, read from a full run (2026-08-20)

This issue's stated acceptance was the text era's row in **section 3** of the data-quality report. The
run finished 2026-08-20 (5,503 s, 32 windows, 0 labels unmeasured):

    era                      award-notices  with lot_results  density  no block parsed
    text 1993–2010               1,306,514           462,772    35.4%          843,753

**0 → 462,772 award notices now carry a result block, 35.4 % density.** When this issue was filed, that
column was the 1.3M-notice hole the whole thing is named after.

And the fold is not the constraint: *"Award notices whose result block IS parsed and still did not
materialise, i.e. the fold's own shortfall: **0**."* Every result block the parse layer holds
materialises. What is left is parse-layer coverage, which is this issue's own remaining work.

### Why 35.4 % and not 91.7 %

The per-package A/B on `fetch 300` put winner coverage at **91.7 % of award notices** — but that is
`fetch 300` re-parsed under slice 3. The corpus-wide number mixes packages re-parsed under slices 1, 2,
3 and 4 at different times, plus ~80 of 216 packages not re-parsed at all yet. The gap between 35.4 %
and ~90 % is exactly the **redo sweep** (task 38: fetch 186–374), which is the campaign's last stage and
is now the only thing between this era and its ceiling.

Section 1 moves too, for the same partial reason:

    era                      versions   title  buyer  value    cpv deadline winner
    text 1993–2010          3,786,955  100.0%  63.0%   0.5%  95.5%    84.4%   13.0%

`winner 13.0 %` against a ~28 % ceiling (91.7 % of awards, awards being ~30 % of the era's versions), and
`value 0.5 %` because the price slices landed only at the very end of the forward pass. Both rise with
the sweep. `buyer 63.0 %` is the separate `AU:`→buyer mapping this issue's header has always named as
riding along with the same re-parse.


## Slice 7 — the sectioned form's money, which is where the era's value actually is

The price arc above was closed on `fetch 300` (2001-06) and I read it as "near its ceiling". That was
true **for the numbered form**. The redo sweep then reached the 2005-2010 packages, and `fetch 200`
(2009-10, notices 3,870,856-3,903,775, 32,920 notices) showed the era's money living somewhere the
slices 4-6 labels never look:

    bodies                                   32,920
    `Total final value`                       9,549
    `INFORMATION ON VALUE OF CONTRACT`        9,152
    `Initial estimated total value`           4,899
    `Value:`                                  9,717
    `Price:`                                     53   <-- the only label slices 4-6 read

    TD:7 award notices                       11,943
    with a winner                            11,485   (96.2 %, slices 2-6 working)
    notice_amounts rows                          41   <-- 0.3 % of the awards

**41 amounts against 9,549 bodies that state the value.** The winner half of this package is at its
ceiling and the money half had barely started, for one reason: `Price:` is a numbered-form label, and
by 2005 the era writes the sectioned EU form instead.

### Why the existing reader could not have read it

The shape is different in kind, not just in wording. Verbatim from prod:

    3870957  Total final value of the contract: | Value: 791 805 EUR. | Excluding VAT.
    3870958  TOTAL FINAL VALUE OF CONTRACT(S) | II.2.1) Total final value of contract(s):
             Value: 39 279 748,48 PLN. | Including VAT. VAT rate (%): 22,00 %. |
             SECTION V: AWARD OF CONTRACT | CONTRACT NO: 1 | V.3) NAME AND ADDRESS...
    3870959  ... Total final value of contract(s): Lowest offer: 16 184 142,63 /
             highest offer: PLN. | Excluding VAT.
    3870962  ... Total final value of contract(s): Value: 54 639 833,00 SEK. |
             SECTION V: AWARD OF CONTRACT | CONTRACT NO: 1
    3870965  ... Value: 104 131,80 EUR. | Excluding VAT. | ... | CONTRACT NO: 4300023446

The numbered form ends its value item with the figure and the next ` <n>. ` marker bounds it. The
sectioned form does **not**: the item continues into prose. `next_item_marker` finds nothing, and the
sub-label retry then strips to after the LAST colon in the window — which in 3870958 is
`VAT rate (%):` and in 3870965 is `CONTRACT NO:`, losing the figure entirely. So these bodies were not
refused for being ambiguous; they were refused for having *more* stated after the number.

### What landed

`TOTAL FINAL VALUE` as a fifth `VALUE_LABELS` entry, and a new `VALUE_STOPS` list that ends the value
item at the first of `EXCLUDING VAT`, `INCLUDING VAT`, `SECTION V`, `CONTRACT NO`, `AWARD OF CONTRACT`
— and, when the stop is a VAT phrase, takes the basis from it. Precedence is nearest-statement-wins: a
marker beside the figure (`HT`, `netto`) beats a sub-label's wording, which beats the stop phrase.

Three things this deliberately does not do:

- **A range claims nothing.** 3870959 is a real body whose second figure is missing at source
  (`highest offer: PLN.`); reading `16 184 142,63` would record the *lowest offer* as the contract
  value. The digit-in-label guard from slice 5 already refuses it, and the test pins that with the
  prod body rather than a contrived one.
- **The basis comes from THIS item's stop.** The first stop wins, so a later lot's `Excluding VAT`
  cannot reach an earlier lot's figure — tested directly.
- **The duplicated label costs nothing.** In the `II.2.1)` flavour the label appears twice, first as
  the section heading. The heading occurrence is refused because its label carries the section
  marker's digits; a refusal does not poison the scan, only a second *claim* that disagrees does.

Falsified rather than assumed: with `VALUE_STOPS` disabled, the new test is the only one in the module
that fails — so the stops, not some pre-existing path, are what read these bodies.


### The slice-7 A/B, and the slice it exposed

Deployed as `8dc4117`, `fetch 200` re-parsed and re-folded (jobs 278/279, 32,920 notices →
30,379 tenders):

    notice_amounts rows, fetch 200        41 → 5,916      (all TED-VAL_TOTAL)
    tax basis captured                    3,690 excl · 1,665 incl   (5,355 of the 5,916)
    bodies stating `Total final value`     9,549, of which 5,893 now carry an amount (61.7 %)

**41 → 5,916.** The first tax-basis data at scale in the corpus, and the first money at all
for this vintage.

Then I sampled the 3,656 bodies that state a value and still yield none, and the top of the
sample looked WRONG — `Value: 37 352 983,46 PLN. Excluding VAT.` is exactly the shape slice 7
reads. Pulling whole bodies explained it, and the explanation is a slice, not a bug.


## Slice 8 — a total and its parts are not a contradiction

Notice 3871013 states its value twice:

    II.2.1)  Total final value of contract(s): Value: 116 250 000,00 SEK.   <- the notice
    V.4)     Total final value of the contract: Value: 116 000 000,00 SEK.  <- one contract

Two `Total final value` claims that disagree, so `awarded_value`'s disagreement rule refused
the notice — correctly, under the assumption that two claims are two readings of ONE fact.
That assumption is wrong for this form. Notice 3871014 settles it arithmetically:

    II.2.1)  Total final value of contract(s):     81 605 403,00 SEK
    CONTRACT NO: 1  V.4)  Total final value:       40 087 596,00 SEK
    CONTRACT NO: 2  V.4)  Total final value:       15 605 000,00 SEK
    CONTRACT NO: 3  V.4)  Total final value:       14 700 772,00 SEK
    CONTRACT NO: 4  V.4)  Total final value:       11 212 035,00 SEK
                                                   ------------------
                                                   81 605 403,00 SEK

The four parts sum to the aggregate **exactly**. `II.2.1` is the notice's total and the `V.4`
figures are its parts, and `TED-VAL_TOTAL` is a notice-scope field — so the aggregate is the
one to claim. The per-contract figures are a `lot_results`-scope fact with no home yet.

The signal is the plural: `of contract(s)` is II.2.1, `of the contract` is V.4. So each label
occurrence now carries a scope, the widest scope the body states wins, and a conflict *within*
a scope is still a refusal — a narrower figure is not a fallback for an unreadable total.

### The correctness half, which matters more than the coverage half

Ranking the scopes exposed a claim that was already wrong. Take a body awarding two contracts
at 40 087 596 SEK each and stating no aggregate: the old rule saw two claims that AGREE, read
them as one fact stated twice, and recorded 40 087 596 as the notice's total — half the real
figure. So a per-contract claim now stands only when the body awards ONE contract.

Counting the contracts took a measurement, and my first rule was wrong. `CONTRACT NO` is a
HEADING in the sectioned form but a REFERENCE in the numbered one — `6. Successful
contractor(s): Contract No 710-7009: AS Anlegg, Arvid` (notice 1710588) — and some pre-2004
bodies mention it twice. Three candidate rules, measured on both bands before choosing:

    rule                         sectioned bodies flagged   pre-2004 amounts dropped
    bare substring                     2,317 of 9,549              7 of 996
    requires a colon (`NO:`)           2,100                       0
    line starts only                   2,154                       0

The bare count drops 7 correct prices per pre-2004 package. Line starts only drops none, and
still flags 2,154 — and 159 of its 163-body gap to the bare count are bodies with ONE heading
plus a mid-line mention, i.e. single-contract notices the bare count would ALSO have refused
wrongly. So the marker is `\nCONTRACT NO`, counted on the raw body rather than the flattened
one, and the numbered form scores 0 in all 996 of `fetch 300`'s amount-carrying bodies.

Both halves falsified separately: with the scope ranking disabled the new test fails with
`None`; with the contract count disabled it fails with `Some(4008759600)` — the understated
claim itself, which is the clearest possible statement of what the guard is for.

Expected payoff, measured before implementing: **1,959 of the 3,656 remaining refusals state
the aggregate with a figure.** The rest are bodies stating no figure at all
(`Total final value of contract(s): Excluding VAT.`), ranges (`Lowest offer: … / highest
offer: …`), and multi-contract bodies with no aggregate — which the correctness half now
refuses on purpose.


### The slice-8 A/B, read on the same package

Deployed as `9236267`, `fetch 200` re-parsed and re-folded (jobs 280/281):

    notice_amounts rows, fetch 200            41 → 5,916 → 7,668
    tax basis captured                        4,853 excl · 2,179 incl   (7,032 of 7,668)
    value-stating bodies with an amount       5,893 → 7,645 of 9,549   (61.7 % → 80.1 %)

Predicted +1,959, actual **+1,752**. The 207-body difference is the correctness half doing its
work — bodies whose per-contract claim the guard now drops, and aggregates that state a figure
this still refuses (below). Predicting a payoff and landing 89 % of it is the right kind of miss:
the direction and the size were both right, and the shortfall has a cause rather than a shrug.

And the basis reaches the canonical layer, verified over notices 3,870,856-3,875,000:

    tender_version_amounts.tax_basis    excl 504 · incl 178 · NULL 73

So issue 251's column now carries real corpus data, not just fixtures — from the text era, which
before slice 7 had no money at all.

### What the remaining 1,904 refusals are

Classified, whole package:

    no aggregate figure, one contract     1,111    no figure stated, or a range
    no aggregate figure, several          644      per-contract parts only — the guard, on purpose
    aggregate figure, one contract        97       see below
    aggregate figure, several             52       see below

The 644 are the correctness half's whole point: a notice that states its contracts' values and no
total does not state a total, and this now says so instead of picking one.

The 149 that DO state an aggregate figure and still refuse are worth a slice, and it is measured
but not yet written:

    3871298   Value: 0,00 GBP.                          refused, correctly — zero is not a value
    3871306   Value: 7 015 000 GBP. | SECTION IV: …     the next heading is SECTION **IV**
    3873797   Value: 99 840 EUR. | SECTION IV: …        same
    3871371   Value: 33 030 818,1 LTL.                  ONE decimal digit, so `digit_group` refuses
    3872503   Value: 176 713,2 RON.                     same
    3872777   Value: 401 320 oltre IVA (per il 3° e 4° lotto) EUR.   prose inside the figure

Two candidate slices, both small and both measured over the package's 1,904 refusals:

- **`SECTION ` rather than `SECTION V` as a stop** — 52 bodies. `II.2.1` sits before section IV, so
  the heading that follows the aggregate is often `SECTION IV: PROCEDURE`, which the current stop
  list does not bound. Generalising the stop to any `SECTION ` subsumes the `SECTION V` entry.
- **A one-digit decimal group as tenths** — 114 bodies. `digit_group` requires exactly two decimal
  digits, so `33 030 818,1` is refused. One digit after the comma is unambiguous (tenths), and
  comma-as-thousands stays refused because a thousands group is three digits.

Together ~166 of 1,904, so ~1.7 % of the package's value-stating bodies — worth doing, worth doing
AFTER the sweep rather than spending a deploy-and-reparse cycle on it while 20 packages wait.


## Slice 9 — two small refusals, one of them my own over-strictness

Both classes named at the end of slice 8, implemented together because they are one line each
and share a test.

**`SECTION ` instead of `SECTION V` as a stop** — 52 bodies per package. `II.2.1` sits before
section IV, so the heading that bounds the aggregate is often `SECTION IV: PROCEDURE`:

    II.2.1)  Total final value of contract(s): Value: 7 015 000 GBP.
    SECTION IV: PROCEDURE
    IV.1.1)  Type of procedure: Open.

With only `SECTION V` in the stop list the figure runs on and the sub-label retry strips to after
`Type of procedure:`, losing it. No section heading is ever part of a value, so the bare word is
the right stop and it subsumes the old entry.

**A one-digit decimal group is tenths** — 114 bodies per package. `digit_group` required exactly
two decimal digits, so `33 030 818,1 LTL` (notice 3871371) and `176 713,2 RON` (notice 3872503)
were refused. That rule was mine, from slice 4, and it sat in the test file directly beside the
sub-cent refusal under the comment *"Sub-cent is ADR-0010's quarantine trigger"* — which is
exactly the confusion: `,255` is sub-cent and unrepresentable, `,2` is twenty cents and exactly
representable. ADR-0010 says nothing about tenths; I grouped them with sub-cent because both were
"not two digits".

What keeps the tenths reading unambiguous is the rule right next to it: a three-digit fraction is
still refused, so comma-as-thousands (`1,000 EUR`, `1,000,000 EUR`) cannot be misread as a decimal.
The only remaining reading of `176 713,2` is 176 713 and 2 tenths.

Falsified separately: with `SECTION V` restored the new body yields `None`; with the two-digit rule
restored `1 000,2 EUR` yields `None`.

Not yet deployed — the 221-240 sweep (jobs 282/283) is running, and a deploy restarts the service,
which re-runs the running job from the top (issue 245). It goes out when the queue is idle.
