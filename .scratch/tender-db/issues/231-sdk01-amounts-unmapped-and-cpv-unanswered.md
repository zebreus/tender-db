# 231 — sdk-0.1 amounts are never mapped, and whether the era carries CPV at all is unanswered

Status: CPV half FIXED in code 2026-08-19 (mapped + fixture-tested, awaiting the era's re-fold); the value
half is NOT a mapping gap — the amounts never reach the parse layer, so its diagnosis moves upstream
Kind: projection mapping gap (one era, two fields) + one research question
Blocked by: —
Relates to: 29 (the parent gap, now verified closed for title/buyer/deadline), 177 (the same
shape one era over: r208 values never projected), 172 (codelist drift across eras), 230 (the
measurement that found this)

## What

Split out of issue 29, whose broad claim ("sdk-0.1 measures 0 % on every field") is no longer
true. Measured over the whole era — **666,671 tender-versions**, not a fixture:

    era                     versions   title  buyer  value    cpv deadline winner
    DÖE sdk-0.1 island       666,671  100.0% 100.0%   0.0%   0.0%    77.9%   1.1%

Two fields remain on the floor and they are different kinds of problem.

## 1. value 0.0 % — a mapping gap, and the cause is already visible in the code

Issue 29's fix comment lists "value fields" among what it repaired, but the changes it then
enumerates touch `TEXTS`, `CLASSIFICATIONS` and `DATES` only. `AMOUNTS` never gained an
`SDK01-*` entry, so sdk-0.1 amounts have never had a canonical destination. 0.0 % is therefore
expected from the code as written — this is not a regression and not a mystery, it is an
unfinished mapping.

Do the archive inventory first (which `SDK01-*` field ids actually carry monetary values, and
at which scope — Tender, Lot, or result), then add the `AMOUNTS` entries, then re-project the
era and re-measure. Issue 177 is the template: it was exactly this, for r208.

## 2. cpv 0.0 % — do NOT map anything until the presence question is answered

Issue 29 mapped `RealizedLocation` NUTS → `place`. That is the *place* classification; CPV was
never in scope. Before any mapping work, answer from the archive: **do sdk-0.1 payloads carry a
CPV code at all?** An era that does not publish CPV is a 0 % that is simply TRUE, and mapping
effort spent against it is spent against nothing.

What the same report says about the neighbours, so the obvious wrong inference is closed off:
`eforms-de-1.1` through `eforms-de-2.1` all measure **100.0 % CPV** over 443k versions
combined. "German notices don't carry CPV" is not the explanation.

## What NOT to conclude from the winner column

`winner 1.1 %` is measured over ALL versions, and most sdk-0.1 versions are contract notices
with no winner to carry. That number cannot separate "winners are lost" from "few of these are
award notices", so it must not be used as evidence either way. The denominator that can is
award notices only — section 3 of the data-quality report (results materialisation), which
job 731 could not measure and which rev `c731a05` onward does. Judge sdk-0.1 winners there.

## Acceptance

- An archive inventory recorded here: which `SDK01-*` ids carry amounts, and whether any carry
  a CPV code.
- `AMOUNTS` mapped for whichever amount ids exist; era re-projected.
- The data-quality report's `DÖE sdk-0.1 island` row shows non-trivial `value`.
- For CPV: either mapped and non-trivial, OR a recorded finding that the era does not publish
  CPV — in which case the 0 % is documented as correct rather than left looking like a bug.


---

## Both halves answered from prod (2026-08-19)

### CPV: the era DOES publish it, so the 0 % was a missing destination

The presence question this issue insisted on asking first, answered before any mapping was written. 175
sampled `can-standard` notices of the era carry **1,328 `cpv`-scheme classification rows** — 7.6 apiece —
under four field ids:

    SDK01-ProcurementProject-MainCommodityClassification-ItemClassificationCode                    147
    SDK01-ProcurementProjectLot-ProcurementProject-MainCommodityClassification-…                   147
    SDK01-ProcurementProject-AdditionalCommodityClassification-ItemClassificationCode              366
    SDK01-ProcurementProjectLot-ProcurementProject-AdditionalCommodityClassification-…             366

So the parse layer had CPV all along and the canonical layer had nowhere to put it — issue 177's shape one
era over, as this issue guessed for the value half. All four ids are now in `CLASSIFICATIONS`, main and
additional, Tender and Lot scope.

**Neither committed sdk-0.1 fixture carries a `CommodityClassification`**, so a test written against them
would have passed against nothing. A real prod payload (`17750180-1`, from the 2022-12 DÖE monthly, 3.9 kB)
is committed as a third fixture and the projection test now asserts that both `main` and `additional` reach
`tender_version_classifications`.

Acceptance for this half: the era's `cpv` column in section 1 of the data-quality report, after the era is
re-folded. It cannot move before that.

### value: not a mapping gap — the amounts are not in the parse layer at all

This issue's reasoning was that `AMOUNTS` never gained an `SDK01-*` entry, so 0.0 % was "expected from the
code as written". Half right, and the missing half changes the work: **`notice_amounts` holds zero rows for
this era.**

    400 sampled sdk-0.1 notices          → 0 amount rows
    175 sampled `can-standard` notices   → 0 amount rows

A single notice's full field inventory shows the shape: texts for party name, project name/description, lot
name/description, city and document references; codes for notice type, country, regulatory domain and
procedure. No monetary value of any kind.

So adding `AMOUNTS` entries would have mapped nothing, and the question moves upstream: does the sdk-0.1
payload carry a monetary value that the PARSER is not claiming as an amount (in which case ADR-0004 says it
is being claimed as something else, and the field inventory above is where to look), or does the dialect
simply not publish one? The `can-standard` sample makes the second answer plausible — an award notice with
no award value at all — but it has not been read from the payloads yet, and that is the next step for this
half.
