# 78 — DÖE eForms serializer nests subtrees the SDK anchors elsewhere (blocks part of the 1.0 reclaim)

Status: open (iterative mapping mop-up; found during the issue-74 1.0 validation)
Kind: completeness / data-quality
Relates to: 74 (SDK 1.0-1.7 vendoring), 75 (eforms-de empirical inventory), ADR-0004, ADR-0002

## Finding (2026-07-29, proj-fix — validating the SDK-1.0 reprocess)

Grabbed 7 real held `eforms-sdk-1.0` members from the DÖE archive (`doe/monthly/`)
and ran them through the deployed parser (46bc21e). **4/7 parse, 3/7 still
quarantine** — but NOT for a grammar reason. The 1.0 grammar + vendoring is
correct; the blocker is that the **DÖE OpenData JAXB serializer emits eForms with
structural deviations** from the schema the EU SDK inventory describes. These are
`eforms-sdk-1.0` (EU) CustomizationID notices, ns-prefixed (`ns3:`…`ns7:`), from
the DÖE below-threshold stream.

Two deviations found so far (each surfaces only after the previous is fixed — the
walker quarantines at the first unclaimed element):

1. **UBO nested under `efac:Organization`.** DÖE puts the whole
   `efac:UltimateBeneficialOwner` (with `efac:Nationality/cbc:NationalityID`, BT-706)
   *inside* `efac:Organization`; the SDK models the full UBO directly under
   `efac:Organizations` and keeps only a reference `cbc:ID` under the Organization.
   → unclaimed `efac:Organization/efac:UltimateBeneficialOwner/efac:Nationality`.
   **Fix (validated):** one `ALIASES` graft
   `efac:Organizations/efac:UltimateBeneficialOwner` →
   `efac:Organizations/efac:Organization/efac:UltimateBeneficialOwner` (gap-fill,
   byte-identical to standard notices, which never nest it). Made the first member parse.

2. **`AppealTerms/AppealInformationParty/cbc:WebsiteURI` unclaimed** (3/7). Needs
   the same treatment (alias/EXTRA to the SDK's landing for that field) — not yet
   fixed; likely one of several more.

## Scope / impact

- **Only the DÖE-serialized stream is affected.** Standard TED-serialized eForms
  parse fine — the issue-74 `can_24_maximal` SDK-1.7 fixture (a TED example with UBO
  + CompanySizeCode) parses exhaustively. So the big buckets (1.7 = 344K, mostly TED)
  should reclaim; this is the small DÖE 1.0 stream (~3.4K) plus any DÖE-serialized
  members at 1.3/1.5/1.6/1.7.
- Not blocking: the deployed vendoring already reclaims the clean 1.0 subset. This
  issue recovers the DÖE-quirk remainder.

## To do

Iterative: for a representative sample of held DÖE `eforms-sdk-1.x` members, fix each
unclaimed-element quirk with an `ALIASES` graft (nesting deviations) or `EXTRA`
(genuinely-extra elements), re-parse, repeat until the sample parses clean. Commit
the graft table + a DÖE-1.0 fixture (or a few) proving exhaustive consumption. Then
reprocess the bucket to reclaim the remainder.

## Note

The mechanism is the same as the existing DÖE-quirk aliases already in `ALIASES`
(the withheld-discriminator and result-layer grafts). This is a bounded mop-up of the
same kind, scoped to the DÖE serializer's eForms output.
