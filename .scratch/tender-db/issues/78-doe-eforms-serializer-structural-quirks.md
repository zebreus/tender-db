# 78 — DÖE eForms serializer nests subtrees the SDK anchors elsewhere (blocks part of the 1.0 reclaim)

Status: DONE — deployed (ae4ec08) 2026-07-29 (DÖE serializer grafts live; reclaim of DÖE-dialect buckets runs on the blanket unknown-customization job).

## Implementation (proj-fix, 2026-07-29)

Iterated a 48-member real DÖE `eforms-sdk-1.0` sample from 21/48 → **48/48 parsing**
with six gap-fill `ALIASES` grafts (byte-identical to standard TED notices, which
never carry these shapes — full store+ingest+app suites green incl. the projection
golden/equivalence/resume gates). The DÖE JAXB serializer deviations found:

1. **UBO nested under `efac:Organization`** (with `efac:Nationality`/BT-706) — the
   SDK models the full UBO directly under `efac:Organizations`. Graft the UBO subtree.
2. **Inlined bodies as full UBL parties** where the SDK references an org by id:
   `cac:AppealTerms/{AppealReceiverParty, AppealInformationParty}` and
   `cac:TenderRecipientParty` (WebsiteURI, PartyName, PostalAddress, Contact). These
   mirror the `efac:Company` shape, so graft Company onto each — at procedure AND Lot
   level (DÖE emits AppealTerms at both). Six grafts total.

Fixtures: two real DÖE 1.0 notices (`doe-sdk10-ubo-appeal`, `doe-sdk10-tenderrecipient`)
in the eForms corpus, exercising the UBO + appeal + tender-recipient grafts; the
nested UBO's nationality is asserted claimed as BT-706. Deploy → reprocess reclaims
the DÖE 1.0-1.7 remainder.

---

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
