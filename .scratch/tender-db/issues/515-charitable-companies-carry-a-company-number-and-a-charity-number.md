# 515 — a charitable company carries a company number AND a charity number: the two never cross-walk

Status: ready-for-agent — triaged 2026-10-11 with issue 469 U2's evidence. NEXT: unit 1, measure (the CC extract's
`charity_company_registration_number` against the standing company-number orgs, offline from the agent container).
Was: needs-triage — filed 2026-10-11 from issue 469's unit-2 decision ("issue T"; its triples are hard-denied).
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

## Verify

    curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' https://tenders.zebreus.click/v1/organizations/31593920

This checks the end state, the cross-walked pair merged, on the issue's own witness from 469 U2: Choice Housing
Ireland = CCNI 100095 = Companies House NP000408. Org 31593920 holds `GBNICNIC100095`. Org 30967375 holds
`GBCOHNP000408` and, as the 448 survivor, the shared PPON, so U4 denied the NIC pair as a triple
(`../469-registry-pairs/u2/registry-sample-2163.json`, entry 91; `u2-results.md` NIC row).

- **done**: `308 https://tenders.zebreus.click/v1/organizations/30967375`. The charity-number org has folded into the
  company-number survivor; 448's alias keeps its owner, as "Proposed unit" says.
- **open** (2026-10-11, from 469 U2's sample of job 2163, not re-read in this pass): `200 `. The NIC org stands apart.
  Unit 1, the measurement, has not started: no commit names 515 after its triage (23c3780), and no code reads
  `charity_company_registration_number` (canonical.rs only defers to "issue 515's cross-walk").

## 469 U2 evidence (2026-10-11)

- The cross-walk source exists and works. The Charity Commission JSON extract
  (`publicextract.charity.zip`, 57 MB) carries `charity_company_registration_number` per charity. U2's fetcher
  downloaded and read it from the agent container. CCNI pages show the company number too: Choice Housing Ireland =
  CCNI 100095 = Companies House NP000408.
- U2's one NIC triple was exactly this shape. `NIC 100095`'s PPON is held by the 448 survivor NP000408, so U4's
  triple gate denies it, as decided.
- U0's `ppon_beside_coh` read 23 triples over the five schemes (CHC 8, MPR 8, UKPRN 5, NIC 1, SC 1). U4 widens the
  input to graph-level (`ppon_with_coh`: any GB party showed the PPON beside a company number), so the first U4 dry
  run gives the real count. Read it there before building.
- MPR societies are ALSO at Companies House (`RS008112` mirrors FCA `8112`, Delta Housing). So the society ↔
  company-number cross-walk is a mechanical prefix rule, not a lookup.
