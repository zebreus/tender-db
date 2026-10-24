# 585 — TenderNed dashboard labels printed as ORG sections are served as organizations

Status: needs-triage — filed 2026-10-24 from issue 576 unit 4's read-back (items 1 and 2). 576 U5 (the `deze
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

## Proposed units

1. **Measure (S)** after 576 U5's re-fold: what roles still bind to the two orgs (tender_version_parties by org, index),
   and from which section kinds.
2. **Decide**: drop ORG sections whose every name is TenderNed dashboard prose (a `not_a_name` class beside 520/525's), or
   leave them served as published (N). Build + sweep if dropped.
