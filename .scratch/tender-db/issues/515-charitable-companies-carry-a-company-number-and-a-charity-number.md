# 515 — a charitable company carries a company number AND a charity number: the two never cross-walk

Status: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision ("issue T"; its triples are hard-denied).
Kind: data quality (organization identity)
Relates to: 469 (PPON beside a company number and a charity number is a denied triple), 448 (the company-number arm)

## What is wrong

A UK charitable company is registered at Companies House and with the Charity Commission; FTS parties publish
either number beside its PPON. The 448 arm keys the company number, 469 will key the charity number, and a PPON
seen beside both is a triple that 469 denies — so the company-number org and the charity org stay two.

## Proposed unit

Use the Charity Commission extract's `charity_company_registration_number` to cross-walk the two identities
(register-confirmed), then admit triples whose survivor is the company-number org (448's alias keeps its owner).
Measure first: `ppon_beside_coh` per scheme on the 469 dry plan.
