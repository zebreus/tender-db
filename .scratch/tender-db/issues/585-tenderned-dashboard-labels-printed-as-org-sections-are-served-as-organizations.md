# 585 — TenderNed dashboard labels printed as ORG sections are served as organizations

Status: ready-for-agent — **unit 1 MEASURED 2026-10-24 ~18:55Z (after 576 U5's Touchpoint re-fold 2768/2769; 4 bounded /v1/sql seeks + 2 GETs)**: the touchpoint mentions are gone (575 → 101 and 370 → 83 mentions). What still binds is the **ORG path of TED R2.0.9 exports** (`ted-export-r209`, NL, notices 22,022,889–23,458,877, 2021–2023): org 18044191 "Vraag- en antwoordmodule in het dashboard van deze aanbesteding" serves **further-information** on 193 version-party rows (section ORG-2, plus 1 ORG-3), org 18044192 "Dashboard van deze aanbesteding" serves **tender-receipt** on 175 (ORG-3); one spelling each. **Census 2780 RAN**: 151 organizations hold the phrase (149 by served name, 1,114 mention rows); roles per Tender: further-information 522, buyer 314, tender-receipt 237, specifications-provider 220, Procedure-Buyer 204, Lot-ReviewInfo 181, Lot-TenderReceipt 148, Contract-Signatory 136, appeal-information 47, winner 3 … So many are REAL BUYERS whose printed name carries the phrase ("… in deze aanbesteding …"): a phrase-contains drop would remove real buyers; the rule must be a whole-name label test (the name IS dashboard/portal prose), with 576 U5's touchpoint use re-checked against these buyer names. The row list was overwritten by census 2781 (one stored report per kind), so census 2792 re-runs it (copy its report before the next census). NEXT: unit 2 decides from 2780 — proposed: drop an ORG-path party whose whole name `portal_label` reads as portal prose (576 U5's phrase), as 520/525's not_a_name, then a refold-fields of the hit notices + sweep. Was: needs-triage — filed 2026-10-24 from issue 576 unit 4's read-back (items 1 and 2). 576 U5 (the `deze
aanbesteding` portal phrase) stops the touchpoint path from binding these labels; this issue is the Organization path,
where the same labels already existed as name-keyed organizations before 576's drain. Next: after 576 U5's deploy and
Touchpoint re-fold, read what still binds to orgs 18044191 / 18044192 and decide.
Kind: data quality / prose served as an organization (W where served as a party)
Relates to: 576 (U5), 483 (buyer census; `portal_label` refuses promotion), 520 / 525 (void-prose names dropped on the ORG
path), 456 (each party serves its own published name).

## Verify

    for o in 18044191 18044192; do curl -s https://tenders.zebreus.click/v1/organizations/$o | jq -c '[.id, .name, .mention_count]'; sleep 2; done

- **open (2026-10-24)**: `[18044191,"Vraag- en antwoordmodule in het dashboard van deze aanbesteding",575]` and
  `[18044192,"Dashboard van deze aanbesteding",370]` (counts as reported by the read-back; they include touchpoint mentions
  576 U5's re-fold retires).
- **done**: neither label is served as a party on any Tender (404 for both orgs after a sweep, or 0 mentions).

## What was found

- Both organizations predate 576's drain (ids ~18.04M, minted from ORG sections). TenderNed notices print these labels as
  the name of an ORG section that carries the buyer's address and TenderNed's URL.
- On the touchpoint path 576 U5 drops them (`touchpoint_gate` → `portal_name`). The ORG path has no such drop: 483's
  BuyerFix keeps a platform name served as published, and `portal_label` only refuses promotion to buyer.

## Unit 1 (2026-10-24, measured)

Reads (bounded: `tender_version_parties_org` and `organization_mentions_org` index seeks, one `notices` PK seek):

| org | name | mentions (was) | role rows | sections | notices |
| --- | --- | --- | --- | --- | --- |
| 18044191 | Vraag- en antwoordmodule in het dashboard van deze aanbesteding | 101 (575) | further-information 193 | ORG-2 192, ORG-3 1 | 22022889–23458877 |
| 18044192 | Dashboard van deze aanbesteding | 83 (370) | tender-receipt 175 | ORG-3 175 | 22022889–23458877 |

Sample notices 22022889 (565872-2021), 22656969 (502713-2022), 23458877 (530958-2023): source `ted`, profile `ted-export-r209`.
So the labels sit in the R2.0.9 address blocks (further information / tenders to be submitted to), where TenderNed printed
its dashboard link text as OFFICIALNAME. 576 U5's gate covers only the eForms touchpoint path.

Census 2780 (queued 2026-10-24): `{"kind":"name-phrase-census","phrases":["deze aanbesteding"],"mentions":true}`.

## Proposed units

1. **Measure (S)** after 576 U5's re-fold: what roles still bind to the two orgs (tender_version_parties by org, index),
   and from which section kinds.
2. **Decide**: drop ORG sections whose every name is TenderNed dashboard prose (a `not_a_name` class beside 520/525's), or
   leave them served as published (N). Build + sweep if dropped.
