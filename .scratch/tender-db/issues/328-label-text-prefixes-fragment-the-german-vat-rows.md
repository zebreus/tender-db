# 328 — "USt-IdNr." glued to the number: label text in the identifier slot fragments 5,766 rows

Status: MEASURED 2026-09-01 — census done, class sized, trap case found. Build
not started.
Kind: data quality / identity (organization layer)
Relates to: 317 (whose Unit C measurement noticed the class and asked for exactly
this census), 325 (the SUFFIX twin of the same phenomenon), 300 (canonical keys)

## Where it came from

Issue 317's Unit C measurement closed with: *"A third class fell out for free:
identifiers carrying German LABEL TEXT (`USTIDDE…`, `USTIDNRDE…`, `HRGNUMMER…`).
That is rule-shaped — a prefix strip in the crosswalk would canonicalize them —
and it is measured here at 4 rows in this band alone. **Worth a corpus census
before deciding.**"

This is that census. The class is **three orders of magnitude larger** than the
band it was spotted in.

## The class

| prefix | rows | | prefix | rows |
| --- | --- | --- | --- | --- |
| `USTID` | **2,869** | | `VATNO` | 45 |
| `USTIDNR` | **1,958** | | `VATNUMBER` | 26 |
| `UMSATZSTEUER…` | **534** | | `REGISTRATIONNUMBER` | 12 |
| `STNR` | 95 | | `CHARITYNUMBER` | 7 |
| `HANDELSREGISTER` | 87 | | `IDNR` | 6 |
| `COMPANYNUMBER` | 63 | | `CHARITYNO` | 5 |
| `STEUERNUMMER` | 57 | | `NIPNUMER` / `HRGNUMMER` | 1 each |

**5,766 rows.** Overwhelmingly German, overwhelmingly `identifier_kind =
'national'`, and the values are perfectly good VAT numbers with a label glued to
the front:

```
USTIDDE329214156      Die Autobahn GmbH des Bundes
USTIDNRDE811335517    Regierung von Oberbayern
USTIDNRDE119272606    Medtronic GmbH
USTIDDE142474721      Gemeinde Bad Bellingen
```

`USTIDDE329214156` is `USt-IdNr. DE329214156`. Nothing is wrong with the number.

## Why it matters: 60.7% have a partner row already standing

Stripping the label and looking for a row carrying the bare core:

| | rows |
| --- | --- |
| **a partner row EXISTS** | **3,253 (60.7%)** |
| no partner | 2,108 |

So this is not cosmetic. **3,253 organizations are fragmented from their own
correctly-formed twin** because one row says `national USTIDDE811740505` and the
other says `vat DE811740505`. They cannot merge, they answer different queries,
and the same buyer appears twice.

The other 2,108 are not wasted work: canonicalising them makes them findable and
merge-eligible as the corpus grows.

## THE TRAP, and it is the same lesson as issue 325 in mirror image

A length-based strip is wrong, because the label vocabulary is **not uniform**:

```
UMSATZSTEUERIDNRDE138117521                    Universität des Saarlandes
UMSATZSTEUERIDDE188369991                      Technische Universität Dresden
UMSATZSTEUERIDENTIFIKATIONSNUMMERDE198235088   Würzburger Institut
UMSATZSTEUERIDENTIFIKATIONSNUMMERGEM27AUMSAT   Kreiskliniken Günzburg-Krumbach
```

Three different label lengths for the same concept — and the fourth carries **no
number at all**. It is the sentence *"Umsatzsteueridentifikationsnummer gem. §27a
UmsatzStG"*, a legal citation the publisher put in the identifier field. Strip a
fixed prefix from it and you invent an identifier out of `GEM27AUMSAT`.

Issue 325 landed on a measured VOCABULARY rather than a length bound for exactly
this reason, on the suffix side (`MVA`, `MWST`, `USTID` — note `USTID` appears at
BOTH ends, the same publisher habit). This class needs the same treatment plus
one extra guard the suffix case did not: **strip, then re-validate**. The
remainder must parse as an identifier, or the row is left exactly as it stands.

## Proposed shape

1. **A measured prefix vocabulary** in the crosswalk — longest-match-first, so
   `UMSATZSTEUERIDENTIFIKATIONSNUMMER` is tried before `UMSATZSTEUERID`. The
   table above is the reading; re-run the census rather than extend it by guess.
2. **Strip then re-validate**: the remainder goes through `normalise_identifier`,
   and only a result that classifies is accepted. `GEM27AUMSAT` classifies as
   nothing and the row stands unchanged. This is the guard that makes the strip
   safe.
3. **Repair the standing rows** with the issue-325 pattern, which is built and
   proven: inject the classifier, re-parse each row, plan the disagreements, dry
   review, wet with `expect_rows` parity, change events (`identifier` and
   `identifier_kind` are published).
4. **Expect collisions and plan for them.** 3,253 rows will land on an identity a
   partner already holds — that is the point — so `match-org-identifiers --r2`
   is a required second step, exactly as in issue 326. The R2 plan is now
   reviewable (issue 326 / `R2_PLAN_LISTING_CAP`), so that step is a normal
   reviewed fold rather than a leap.

## What NOT to do

Do not stop at the strip and call it done. The value of this class is the 3,253
REUNIONS, and those only happen when the fold runs afterwards.

Do not extend the vocabulary by imagination. `HRB`/`HRA` look like the same
shape and are **not** — those are genuine German register references where the
letters carry meaning, unlike `USTID` which is pure label. The census
distinguishes them; a guess would not.
