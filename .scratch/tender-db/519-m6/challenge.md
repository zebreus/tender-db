# 519 M6 — adversarial challenge of the prepared cohort

Challenged 2026-10-24 18:40–18:57Z against `build-specs/519.md` (incl. its Check section), the rubric, and
`519-nhs-digit-codes/unit2-decision.md` §4. Inputs: `verdicts.json` (POST body, sha256 `e259a87a…50cdb5`, 245 rows),
`review.json` (sha256 `0f9450d8…7d056a`, 248 rows), `reading.md`, `live.json`, `raw/sql/*`, `register/*`, `api/*`.
Nothing was posted, no git, no cargo. Files written: this one, `raw/sql/c1_salted.{sql,plan,out}` (+ the
`raw/state/sql_state.json` ledger entry), `api/c_*.json`.

## Verdict

**NO-GO for the body as built. GO once C1 is applied** (one row, high → medium) and `verdicts.json` is rebuilt
with `tools/post_body.py`. C2 is recommended in the same rebuild, but it does not block: an unkeyed correction
never re-keys. Everything else is advisory.

## Changes

### C1 (blocking): 31623642 `GBNHSY61EASTOFENGLANDCOMMISSIONINGREGION` "Vaccination UK Limited": change merge/high to hold/medium

As built, the row merges into 30968917 (`GBCOH03682679`, VACCINATION UK LIMITED).

- **What the value is.** The raw identifier is `GB-NHS-Y61 EAST OF ENGLAND COMMISSIONING REGION`. That is the label of
  an active NHS England body: ODS Y61 "EAST OF ENGLAND COMMISSIONING REGION", RO209, Active
  (https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/Y61). It is not junk and it is not this
  company's identifier: it is another body's real code. Y61 is on decision §4's last row ("Real codes on the
  wrong holder … Not unit 2").
- **Why the merge is harmful.** The company match itself is right: both rows are Vaccination UK Ltd, register
  03682679 (`register/03682679.json`). The problem is the alias the merge leaves behind.
  - A rekey stamps the wrong triple. From then on, every later mention under that exact triple binds to the
    target through `rekey_target` (canonical.rs:15659, resolver call at :23200). That bind does no name check.
  - Future NHS England East of England mentions under this label would therefore land on Vaccination UK's
    canonical, unflagged row.
  - If the row is not merged, the exact triple binds to 31623642, which serves `register_mismatch`.
- **Reuse is the observed pattern.** The sibling labels already carry several bodies' mentions:
  - `GBNHSY62NORTHWESTCOMMISSIONINGREGION` (30914099): 17 mentions of L&SC ICB, GM ICB, C&M ICB and NHSE NW;
  - `GBNHSY63NORTHEASTANDYORKSHIRECOMMISSIONINGREGION` (31603424): 9 mentions of NHSE NEY, ICBs and trusts.
- **Consistency.** The reader held the identical case at medium: 31623760, Gateshead under the Y63 label, "the
  value is the typed label of NHS England region Y63, another body". The rubric also says a merge is never high
  when another body is involved.
- **Fix.** In `review.json` set `confidence: medium`, `expected: {shape: hold, target: 30968917}` and
  `challenger_agrees: false`. Keep `03682679` as the correction (withheld and flagged, never re-keyed). Then rebuild:
  `python3 -I tools/post_body.py review.json live.json 519-m6-2026-10-24 verdicts.json`.
- **Effect on the runbook numbers:**
  - merges 87 → 86 and N 100 → 99;
  - `not_high` +23 → +24;
  - corrected rows: high 114 → 113, medium 23 → 24;
  - case-reviews: `('wrong','medium',False)` = 24, high rows = 221;
  - the fallback refold list drops 31623642's one notice (rebuild it with `tools/notices.py`).

### C2 (recommended, non-blocking): 31613184 `GBNHSGREATERMANCHESTERCOMMISSIONINGHUB14J`: drop the correction `14J`, so it becomes wrong/none

- 14J is not an NHS England code. ODS has it as "GREATER MANCHESTER COMMISSIONING HUB", RO98 (the CCG/sub-ICB
  role), Inactive since 2021-09-30, successor 27T
  (https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/14J).
- The rubric's NHS England clause ("the NHS England code if the value spells one") is therefore not met. The
  mentions name "NHS England Greater Manchester" (2).
- The only GB-NHS carrier of 14J is 31604394, "NHS Greater Manchester Integrated Care Board", which is a different
  body. Under decision §4's optional U8 `GB:nhs` key, this correction would point the NHS England GM row at the
  GM ICB.
- Today the correction is `unkeyed` and `correct_identifier` is not served by `/v1` (read.rs and v1.rs never read
  it), so nothing moves.
- Effect: `unkeyed` +14 → +13; high-corrected −1; high-bare +1.

### C3 (follow-up cohort, not this POST): company numbers missed on "junk" rows

These rows are posted wrong/none/high. That verdict is correct and safe: the literal is no ODS code. But each value
is a register number with its leading zero dropped, and the review never ran the register check:

| org | literal | register (8-char) | mention | reading |
|---|---|---|---|---|
| 31603349 (salted) | `GBNHS7816931` | 07816931 KESSON PHYSIOTHERAPY LIMITED (https://find-and-update.company-information.service.gov.uk/company/07816931) | Kesson Physiotherapy Service | same company |
| 31593622 | `GBNHS9086757` | 09086757 HINCKLEY & BOSWORTH MEDICAL ALLIANCE LIMITED (…/company/09086757) | Hinckley & Bosworth Medical Alliance | same company |
| 31603371 | `GBNHS8159149` | 08159149 SOUTH EAST ULTRASOUND LIMITED (…/company/08159149) | Southeast Ultrasound | same company |
| 31603350 | `GBNHS8325113` | 08325113 PHYSIOLOGIC PAIN RELIEF SOLUTIONS LIMITED, Hythe (…/company/08325113) | Physiologic Hythe Ltd | name differs: medium at most |

A later re-POST adds the correction by upserting the same triple. It must set the route to `coh-value` first,
because post_body's Check-fix-4 guard refuses to pad a number on any other route. The other seven-digit or
glued "junk" rows were not name-searched (reading deviation 7). More such corrections may exist. None is harmful
as posted.

### C4 (advisory): medium holds carrying a PPON that D2 says sites and places never take

- 31624630 `GBNHS16H` (NENC ICB plus "Tees Valley Place");
- 30974733 `GBNHSUKF2` (UHL plus the site "Leicester Royal Infirmary").

At medium these never re-key, so the POST is unaffected. Drop the correction, or mark both rows so that the U8a
"re-POST the holds at high" step never promotes them. Do the same for 31544639 `GBNHSQM8`: its hold target
31554399 has 1 mention and is not the QM7 survivor.

## Residual risks (no change required)

- **The alias reaches generic values.** The merges of `GBNHSUK1LONDON` (31606280), `GBNHSUKF22`, `GBNHSUKG23` and
  `GBNHS1111` make any later mention under the same NUTS code or short number bind to the target PPON org. There is
  no name check (as in C1). Spec D2 sanctions this. Each value has exactly one mention in the whole corpus;
  `GBNHSUKN06` has 11, all NIBTS. Hold UK1LONDON at medium if you want to be conservative.
- **31597984 `GBNHS1051858` (WHH): charity-number shaped, unverified.**
  - The value looks like RWT's charity number 1059467, which the reader held at medium for exactly that reason.
  - It cannot be checked here:
    - https://register-of-charities.charitycommission.gov.uk/charity-search/-/charity-details/1051858 answered 403;
    - https://findthatcharity.uk/charity/1051858 answered 502;
    - no org carries it (`/v1/organizations?identifier=GBCHC1051858` → 0 items, while the control GBCHC1059467 → 31601629).
  - If it proves to be the trust's charity, move it to medium.
- **Y62 (31613185) and Y54 (31619926): rubric-compliant corrections whose only GB-NHS carriers are miscoded holders.**
  - The carriers are 31544003 East Lancashire Hospitals and 31593336 York & Scarborough.
  - Y54 is inactive. ODS ended it in 2019/2020, with successors Y62 and Y63
    (https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/Y54).
  - Q76 is inactive too (https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/Q76).
  - Unkeyed today. Any future `GB:nhs` key must never use these corrections without the name gate.
- **Name-gate residue to expect as plan_diff NOTEs:**
  - 31627918, whose target 14647720 heads under the former name "Allied Health-Services…". Its own mentions are
    25/33 "Care Quality Services…", so the gate likely admits it.
  - 31599268, 31544155 and 31608238 (a bullet and a tab in the mention name).
- **Successions, for awareness:** RY2 Bridgewater has an ODS successor, RWW, since 2026-04-01. RY3's predecessor
  is RYV (https://directory.spineservices.nhs.uk/ORD/2-0-0/organisations/RY2, …/RY3). Both corrections stay right.
- **Body splits stay standing.** Several M6 targets' bodies also hold a standing own-code GB-NHS row outside
  `code_survivors`, for example GBNHSR1F 31542853, GBNHST6Y0W 30919647, GBNHSQK1 30916103 and GBNHSRX1 31540199. M6
  picks the PPON tier per D2, which leaves the split for 469 U7 or later. This is not an M6 error.

## What was verified (whole cohort)

- **Owners.** Every keyable correction (the 100 high merge/move rows plus the medium holds that name a target) has
  exactly one standing, non-provisional owner in r4/r7/r8/r9, and it is the reviewed target. No reviewed move has an owner.
- **No earlier verdicts.** No cohort org or merge target carries an earlier 452 verdict
  (`raw/state/prior_identifier_verdicts.json`, 1,199 rows).
- **Scope (§3.1).**
  - All 245 posted orgs are class E or F in `gbnhs-orgs-classified.json`, or the new ODS 404 `GBNHSSLA20`.
  - None is in `sub-icb.json`.
  - None carries a bare miscoded real code (RJ7, Y54, Y61, Y62, RH8C3, D8D1G).
  - D8's 31623641, 31610156 and 31610157 are absent.
- **Handler limits.** The longest literal is 64 characters; admin.rs:474 refuses only > 64. The longest rationale is
  639 bytes. Every correction is 1–64 characters or null. Verdicts and confidences are all in the allowed values.
- **QHL15E.** 31601033 is posted high as GBPPONPWQY9673JBYN (Check fix 2).
- **Exclusions.**
  - No NHS England row carries a PPON.
  - The site, hosted, network and collaborative rows carry no correction: 86101, UKC14, SYB Pathology and the LCW
    Unscheduled Care Collaborative (the West Yorkshire collaborative is a D8 row).
  - The host-trust PPON on SYB Pathology is wrong/none.
- **PPON merges.** All 48 high PPON merges target NHS bodies (trusts, ICBs, NIBTS) whose own names are on every
  mention. No PPON route lands on a non-NHS body.
- **Company merges and moves.** All 52 high ones are register-backed (`register/*.json`): the current or a previous
  name is the org's. The salted ones were re-fetched, below.
- **Spelled codes.** ODS was spot-read for the codes the values spell. Each is the body named:
  - T6Y0W = NHS Norfolk and Suffolk ICB (…/T6Y0W);
  - G6V2S = North London NHS FT (…/G6V2S);
  - RY3 = East of England Community Health and Care NHS Trust;
  - RY2 = Bridgewater.

## Salted 20 (salt `519-m6-check`: the first 20 of the 245 posted rows by sha256("519-m6-check:"+org_id)), end to end

- **Live re-read.** One `/v1/sql` statement, `c1_salted`, at 18:54:31Z: 129 rows in 0.38 s, not truncated. It is a
  PK read of the 20 orgs plus 8 targets/peers, UNION ALL their mentions.
- **Plan checked locally first.** EXPLAIN on `577-triage/plan-copy.db` (sqlite, opened read-only and immutable) and
  turso `plan-probe` on its byte copy (sha256 `80391c50…ce51`) and on `plan/plan-idx.db`. Every step was a SEARCH:
  the rowid PK, then `organization_mentions_org`. The SQL went on STDIN through `tools/sq.py`.
- **Public API.** 10 GETs, serial, at least 2.1 s apart (`api/c_*.json`).

| org | posted | re-check | result |
|---|---|---|---|
| 31603387 `G7114953` | wrong/none high | standing, literal and names unchanged; G-number, not ODS | agree |
| 31603349 `7816931` | wrong/none high | unchanged; register 07816931 = KESSON PHYSIOTHERAPY LIMITED | agree (missed correction, C3) |
| 31627905 `04251815` | merge → 30974841 high | unchanged; register ASHWOOD HOME CARE LIMITED (…/04251815); target stands `GBCOH04251815`, mention ASHWOOD HOME CARE LIMITED | agree |
| 31593802 RY2 label | unkeyed RY2 high | unchanged; ODS RY2 = Bridgewater (Active, successor RWW 2026-04-01) | agree |
| 31571914 glued F-codes | merge → 31597495 high | unchanged; register ST HELENS PHARMACY LIMITED (…/08714511); target stands `GBCOH08714511` | agree |
| 31571857 `FLP04FVP39` | wrong/none high | unchanged (6 mentions, Care Pharmacy Ltd) | agree |
| 31598400 `FKQ07FH658` | wrong/none high | unchanged | agree |
| 31550843 `PHHT9556BWJJ` | merge → 31535216 high | unchanged; target stands `GBPPONPHHT9556BWJJ` NELFT (314 mentions, merged GBNHSRAT) | agree |
| 31607981 `G6782531` | wrong/none high | unchanged; a collaborative, so no correction (D2) | agree |
| 31571845 glued F-codes | wrong/none high | unchanged (5 mentions) | agree |
| 31619925 Q76 label | unkeyed Q76 high | unchanged; ODS Q76 = NHSE Midlands (North Midlands), Inactive | agree (inactive code noted) |
| 31608238 `QN1` | merge → 31542352 high | unchanged; target stands `GBPPONPCLY6219YPTV` LLR ICB (10 mentions, all LLR ICB) | agree |
| 31534565 `11911171` | hold → NELFT medium | unchanged; register CARE CITY INNOVATION C.I.C. | agree |
| 31575414 `YL02356` | wrong/none high | unchanged | agree |
| 31536836 `PYZV3517VQCJ` | wrong/none high | unchanged; host-trust PPON on a pathology network | agree |
| 31571832 glued F-codes | wrong/none high | unchanged | agree |
| 31619926 Y54 label | unkeyed Y54 high | unchanged; ODS Y54 Inactive (successors Y62/Y63) | agree (inactive code noted) |
| 31627918 `11696513` | merge → 14647720 high | unchanged; register CARE QUALITY SERVICES LIMITED, previous name ALLIED HEALTH-SERVICES LIMITED 2018–2023 (…/11696513); target `11696513`, 25/33 mentions Care Quality Services | agree (name-gate NOTE possible) |
| 31572041 glued F-codes | wrong/none high | unchanged | agree |
| 31571838 `FFN74FR433` | merge → 30916889 high | unchanged; register DAY LEWIS PLC (…/01202866); target `GBCOH01202866`, 12 Day Lewis PLC mentions | agree |

Result: 20/20 still stand with the R1 literal and the R2 mention names, and every target stands with the reviewed
literal. All 20 posted verdicts agree with the re-check. One posted verdict misses a correction, but it is still
right as posted (C3).

## Budget

- **`/v1/sql`: 1 statement** (≤ 4 allowed). It is the 519 M6 ledger's 17th of 23, so 6 remain, which covers
  Check item 16's R2 re-read (≤ 4). 0 × 408.
- **Public API: 10 GETs.**
- **WebFetch:**
  - ODS: Y61, Y54, Q76, 14J, RY2, RY3, T6Y0W, G6V2S;
  - Companies House: 11696513, 04251815, 08714511, 01202866, 07816931, 08325113, 08159149, 09086757, CE003108
    (charitable incorporated organisation, WARRINGTON SPEAK UP);
  - Charity Commission 1051858: 403; findthatcharity 1051858: 502.
