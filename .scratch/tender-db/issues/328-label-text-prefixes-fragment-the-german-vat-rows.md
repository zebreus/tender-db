# 328 — "USt-IdNr." glued to the number: label text in the identifier slot fragments 5,766 rows

Status: REPAIRED 2026-09-01 — prevention (`5f64ec7`) and repair (job 565, 5,309
of 5,309 applied) both live. **One claim in the original write-up was WRONG and
is corrected below: the 3,270 "reunions" are not merges R2 will perform.**
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

## PREVENTION LIVE (2026-09-01, `f74da37`)

`normalise_identifier` now strips a leading publisher label and re-parses.
`USTIDDE329214156` and `DE329214156` produce the identical identifier
(`vat DE329214156`), which is the property the whole class turns on — without
it the 3,253 rows stay fragmented from their twin.

The vocabulary is read off the corpus and is richer than the first guess:

```
USTIDDE 2,560   USTIDNRDE 1,840   UMSATZSTEUERIDDE 227   USTID 178
UMSATZSTEUERIDENTNRDE 111   UMSATZSTEUERIDENTIFIKATIONSNUMMERDE 93
STNR 77   STEUERNUMMER 48   USTIDNR 45   USTIDNUMMERDE 19
USTIDNRATU 9   USTIDATU 5   HANDELSREGISTERHRB 9   USTIDNRDEDE 3
```

`USTIDNRATU`/`USTIDATU` are the reason the strip is country-agnostic — Austria's
`ATU` prefix has to survive intact — and `USTIDNRDEDE` is a doubled country code
that must NOT be rescued into something plausible.

**Strip then re-validate, and the recursion is the re-validation**: the remainder
goes back through `normalise_identifier` and is accepted only if it classifies.
Issue 325's suffix work needed no such guard because a label at the BACK sits
behind a value that already parsed; a label at the FRONT hides the value entirely
until it is gone.

### Two things my own tests caught

* **The first implementation invented an identifier.** `find_map` skips a longer
  entry whose remainder is empty and falls through to a shorter one, so the bare
  field name `UMSATZSTEUERIDENTIFIKATIONSNUMMER` became
  `ENTIFIKATIONSNUMMER` — exactly the fragment-invention the doc comment warns
  about, in the function the comment is attached to. Fixed to pick the longest
  match FIRST and judge the remainder second.
* **The guard test's fixtures were reading the gate, not the strip.** They used
  `…123456789` and `…12345678`, which the v2 gate condemns as ascending runs.
  Issue 325's own test carries a written warning about this trap and I walked
  into it again one file over.

### `HANDELSREGISTER` is correct and currently inert

It strips to `HRB12345` — right, because `HANDELSREGISTER` is a field name and
`HRB` is the register division — but the gate condemns bare `HRB…` values
anyway, so both forms return `None` and those 96 rows are unchanged either way.
Pinned as a documented no-op so nobody "fixes" it into working.

## The repair needs its own job, and here is why

`repair_minted_countries` (issue 325) is the right PATTERN — inject the
classifier, re-parse each standing row, plan the disagreements, dry review, wet
with `expect_rows` parity — but it cannot be reused directly: it walks
`identifier_kind = 'vat'`, and **every row in this class is `national`.** That
scope is load-bearing there (it is what makes that job idempotent), so widening
it would trade a proven property for convenience.

So the repair is a sibling job with the same ladder and a different walk:

1. A primary-key pass over identifier-bearing rows, filtered in Rust by
   `label_prefix_stripped` — no index leads with `identifier`, so this is one
   sequential scan, the same shape `country-cluster-census` already runs at this
   size.
2. Re-parse, plan the disagreements, dry review, wet with parity and change
   events (`identifier` and `identifier_kind` are both published).
3. **Then `match-org-identifiers --r2`**, because 3,253 rows will land on an
   identity a partner already holds and the reunion IS the value. That plan is
   reviewable now (issue 326 / `R2_PLAN_LISTING_CAP`), so it is a normal step.

## REPAIR APPLIED — and a claim I got wrong

`repair-label-prefixes`, dry-reviewed twice and applied: **5,309 of 5,309, 0
skipped.** 5,602 rows carried a label; 293 already agreed with the re-parse.
Only 161 `USTID*` values remain, and those are the ones the guard correctly
refuses.

### The dry review caught my own prevention mangling identifiers

The FIRST dry plan contained these:

```
UMSATZSTEUERIDENTIFIKATIONSNRENTEGAPLUSGMBHDE813810149
    -> ENTIFIKATIONSNRENTEGAPLUSGMBHDE813810149
HANDELSREGISTERNRHRB64128       -> NRHRB64128
HANDELSREGISTERARNHEM09155985   -> ARNHEM09155985
```

The vocabulary carries `UMSATZSTEUERIDENTIFIKATIONSNUMMER` and
`UMSATZSTEUERID` but not `…SNR`, so a SHORTER entry matched and left a fragment.
The longest-match fix only covers the case where the exact entry leaves NOTHING;
it cannot help when a shorter one leaves something plausible.

**The real defect was the guard, and it was mine.** "Strip then re-validate"
validated nothing: `normalise_identifier` almost never returns `None` for a
string containing a digit, because `national()` is a catch-all. I wrote that
guard, documented it as the thing making the strip safe, and it was doing no work.

The remainder must now be RECOGNISABLE — a real scheme, or pure digits (a
registration number that lost its label, the `STNR`/`STEUERNUMMER` class). That
makes the vocabulary's GAPS SAFE, which is the property that matters for a list
read off a growing corpus. Second dry plan: **0 unrecognisable remainders**, all
four known fragments gone, `rows` 5,548 → 5,309 with the 239 difference moving
into `already_clean`.

### THE WRONG CLAIM: those 3,270 are not merges

This issue said the reunions were "where the value is" and that
`match-org-identifiers --r2` would perform them. **It will not.** Measured after
the wet run: 3,215 exact duplicate `(DE, vat, DEnnnnnnnnn)` triples now stand —
`DE329214156` is held by three "Die Autobahn GmbH des Bundes" rows — and R2's
dry plan contains **none of them**. Its 18 planned groups are ordinary FR/FI/IT
accumulation, unrelated to this repair.

The reason is deliberate and pinned: `crosswalk::canonical_key` has **no German
arm at all**. From its own must-NOT panel:

```rust
// DE has no cross-walk at all — court-scoped registers.
assert_eq!(key(Some("DE"), "vat", "DE136695976"), None);
```

So German rows were never E1-keyed, before this repair or after. The `reunions`
counter measures **duplicates created**, not merges that will happen, and I
attached the wrong conclusion to a correct number. The job's summary line and the
store docs now say so explicitly rather than misleading the next reader.

### What the repair DID buy, stated honestly

* 5,309 rows carry the identifier the publisher actually meant, with `vat` where
  it belongs instead of `national` on a label-prefixed string.
* The fragmentation is now **visible**: 3,215 exact-duplicate triples, where
  before the same organization sat under two *different* values and nothing could
  see the pair.
* The published string is untouched — `organization_mentions.raw_identifier`
  still holds what each notice said.

### The open question, and it is a real one

Folding those 3,215 needs a DE arm in `canonical_key`, and there are honest
arguments on both sides:

* **For**: `DE:vat` is a HARD scheme, the checksum is strong, and 3,215 exact
  triples with matching names are as clean a merge signal as this corpus offers.
* **Against**: a German VAT number can be shared across an **Organschaft** (a
  fiscal unity of legally distinct companies), which is exactly the false-merge
  shape the CZ699 group-VAT negative already guards against. And the pinned
  negative's stated reason — court-scoped registers — is about `HRB`, not VAT, so
  the two halves of that assertion may deserve different answers.

That is a measurement, not a judgement call to make from the armchair: count how
many of the 3,215 have DISAGREEING names, which is the Organschaft signature.

### The open question is now issue 329

Filed 2026-09-01 as **329 — Unfoldable duplicate identities, and whether
`canonical_key` should get a DE:vat arm**, with the `duplicate-identity-census`
job built to answer it. Read that issue rather than re-deriving the question
here: the measurement it runs is corpus-wide over every `(country, kind)` scope
with no cross-walk arm, not DE-only, and it records why `contained` is an
undecided bucket rather than a licence to fold.
