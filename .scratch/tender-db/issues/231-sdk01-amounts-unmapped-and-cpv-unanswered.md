# 231 — sdk-0.1 amounts are never mapped, and whether the era carries CPV at all is unanswered

Status: CPV HALF **CLOSED** 2026-08-20 (the era measures **cpv 93.8 %**, up from 0.0 %). VALUE HALF
**FIXED IN CODE** 2026-08-20 — and its recorded diagnosis was WRONG: the amounts DO reach the parse
layer, under `SDK01-*` ids, so this was a missing canonical destination exactly like the CPV half.
Three ids added to `AMOUNTS`, gated by a new era-matrix row and falsified. Awaiting deploy + an
sdk-0.1 refold for the acceptance number
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


### CPV verified on prod after the re-fold

`refold eforms:eforms-sdk-0.1` (job 917) re-queued **667,084 notices** and the paired fold (job 918) wrote
**656,503 tenders / 668,913 versions**. Spot-checking a tender of the era afterwards:

    tender 1,746,372:  cpv additional 4 · cpv main 2 · nuts place 2

Before this the same rows read `nuts place` only. So the era's published CPV now reaches the canonical
layer at both scopes, and the next data-quality run should move its `cpv` column off 0.0 % — that column is
the acceptance number and it is the one thing still to read.


---

## ACCEPTED (2026-08-20): the era's cpv column, read from a full run

Section 1 of the data-quality run completed 2026-08-20 (5,503 s, 23 eras, 32 windows, 0 labels
unmeasured):

    era                       versions   title  buyer  value    cpv deadline winner
    DÖE sdk-0.1 island         667,084  100.0% 100.0%   0.0%  93.8%    77.9%    1.1%

**cpv 0.0 % → 93.8 %.** That is the number this issue said could not move before the era was re-folded,
and the one thing it was still waiting on. The CPV half is closed.

The remaining 6.2 % is not this issue's business: an sdk-0.1 notice that publishes no
`CommodityClassification` has no CPV to project, and the presence question this issue insisted on
asking first is what established that the era does publish it *when it has one*.

**The value half stays open, unchanged.** Still 0.0 %, and still not a mapping gap: `notice_amounts`
holds zero rows for the era, so there is nothing for an `AMOUNTS` entry to catch. The open question is
the one recorded above — whether the payload carries a monetary value the parser is not claiming, or
the dialect publishes none.


## The value half — my diagnosis was wrong (2026-08-20)

What this issue recorded, twice, in its own words: *"still not a mapping gap: `notice_amounts` holds
zero rows for the era, so there is nothing for an `AMOUNTS` entry to catch"*, and the open question was
*"whether the payload carries a monetary value the parser is not claiming, or the dialect publishes
none"*.

Neither. The payload carries monetary values AND the parser claims them. I probed every committed DÖE
fixture through the real parse path and printed each claimed `Amount`:

    doe-sdk01-ple-addinfo      SDK01-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount     262 701,78 EUR
    doe-sdk01-ple-addinfo      SDK01-ProcurementProjectLot-ProcurementProject-…-EstimatedOverallContractAmount  262 701,78 EUR
    doe-sdk01-subcontract      SDK01-ProcurementProject-RequestedTenderTotal-TotalAmount                         87 000 000,00 EUR
    doe-sdk01-subcontract-rate SDK01-ProcurementProject-RequestedTenderTotal-TotalAmount                          1 307 200,00 EUR
    doe-sdk01-subcontract-rate SDK01-TenderResult-AwardedTenderedProject-LegalMonetaryTotal-PayableAmount        7 × per-tender
    doe-sdk01-subcontract      SDK01-TenderResult-SubcontractTerms-Amount                                         5 366 643,00 EUR

Every one is a `NoticeValue::Amount` in the parse layer. `AMOUNTS` knew `BT-27`, `BT-271`, `BT-161`, the
legacy `TED-*` ids and two `UBL-*` grafts — and no `SDK01-*` id at all. So `value 0.0 %` was the same
defect as `cpv 0.0 %`, in the same table family, and the "zero rows in `notice_amounts`" claim that sent
me looking upstream was simply not true for notices that publish a `RequestedTenderTotal`.

How the wrong claim survived: the era's row in the projection matrix
(`every_era_projects_its_headline_fields`) has `value: false`, and I read that as "the era has no value
to project". It means "THIS FIXTURE carries none" — `doe/sdk-0.1-numeric-cn-25599482-1.xml` has no
`RequestedTenderTotal` at all. The matrix was honest; my reading of it was not, and it is the same
mistake as reading a missing report row as a zero.

### What landed

Three ids into `AMOUNTS`, all to `estimated_value`:

    SDK01-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount
    SDK01-ProcurementProjectLot-ProcurementProject-RequestedTenderTotal-EstimatedOverallContractAmount
    SDK01-ProcurementProject-RequestedTenderTotal-TotalAmount

Both element spellings map to the estimate because the draft-era publishers used them
interchangeably — `ple-addinfo` carries `EstimatedOverallContractAmount` and no `TotalAmount`,
`subcontract` carries `TotalAmount` and no `EstimatedOverallContractAmount` — inside the same
`cac:RequestedTenderTotal` container, which is the request side, never the award. Scope comes from the
value's section, so the lot-level id lands on the lot with no extra rule.

Two ids deliberately NOT mapped, recorded here so the next reader does not have to re-derive it:

- `SDK01-TenderResult-AwardedTenderedProject-LegalMonetaryTotal-PayableAmount` — the awarded value per
  tender, which is a results-graph fact (BT-720's shape). Routing it into `AMOUNTS` would file every
  award value as a tender estimate. It belongs with whatever unit takes on the era's award side, and
  one number there needs care before anybody calls it a gap: section 1 reads **winner 1.1 %** for this
  era, but that denominator is ALL versions, and I have not measured what share of the era's 667,084
  versions are award notices at all. If sdk-0.1 is overwhelmingly contract notices, 1.1 % may be at or
  near its ceiling — section 3's award-notice count for the era is the number that settles it, and the
  committed fixture evidence (one CAN whose winner DOES resolve, in
  `sdk01_projects_title_buyer_and_winner`) is consistent with either reading. Measure before filing.
- `SDK01-TenderResult-SubcontractTerms-Amount` — the subcontracted share, which has no canonical home
  in any era (the same shape as the text era's per-contract values, issue 244 slice 8).

Gated by a second sdk-0.1 row in the era matrix, on a fixture that DOES carry the element, with the
value column true. Falsified: removing the two `EstimatedOverallContractAmount` ids fails that row with
*"the fixture carries a value and the canonical layer lost it"*.

### What is still needed for the acceptance number

A **refold** of the sdk-0.1 island (not a re-parse — the parse layer already holds the amounts), then
section 1's `value` column for `DÖE sdk-0.1 island`. The CPV half moved 0.0 % → 93.8 % on exactly that
path. Unlike CPV, the ceiling here is unknown: I have no measurement of how many of the era's 667,084
versions publish a `RequestedTenderTotal` at all, and the fixture evidence (two of four sdk-0.1
fixtures) is far too thin to extrapolate from. The refold's own number is the measurement.

## 2026-08-27 — the value refold RAN (owner, overnight window)

Jobs 388/389 (admin `refold` kind, profile `eforms:eforms-sdk-0.1`, expect guard
passed at 669,265 notices vs ~650k estimate): refold 1298 re-queued 669,265
notices + stamped 658,646 tenders epoch-stale; the paired incremental fold
completed in 2,022s — 658,646/658,646 Tenders folded, 671,105 versions written,
0 retired, health green throughout. The acceptance number (section 1's `value`
column for the DÖE sdk-0.1 island) rides TODAY's scheduled weekly data-quality
run — read it when the run lands and close the VALUE half against it. The
ceiling remains unknown by design (the refold's own number is the measurement).
