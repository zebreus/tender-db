# 452 — organizations carry company numbers that Companies House gives to another company, a dissolved shell, or no one

Status: ready-for-agent — filed 2026-09-30 from issue 448's review campaign. Next: size the class across every GB
company number the corpus holds, then decide what a wrong number does to the org (hold it back from matching, flag
it on the API, or both).
Kind: data quality (identifiers)
Relates to: 448 (the altid campaign found them), 327 (the Austrian GLN on supplier rows: the same shape, a buyer's
number on a supplier), 440 (OJS notice numbers accepted as identifiers)

## What was found

Issue 448's campaign checked 4,872 company numbers against the public Companies House pages. Among the 836 pairs
the reviewers read, these planned pairs carry a company number that is not the named organization's:

| number | register says | notices name |
|---|---|---|
| 02202746 | no such company (404) | Harvey Nash Limited, whose own number is 02202476 (transposed digits) |
| 02905600 | no such company (404) | VP-AV Limited (02095600, transposed digits) |
| 36389580 | no such company; outside the issued range | Caretech UK Ltd |
| 11268329 | THINK UP MARKETING LIMITED, dissolved | London Borough of Barking and Dagenham |
| 11747311 | SOUTH TEES DEVELOPMENTS LIMITED | Tees Valley Combined Authority, a statutory body with no company number |
| 11386208 | LEGACY FINANCE GROUP LIMITED, dissolved | Limms Care Services Ltd |
| 13322491 | CS CLIFTON LIMITED, dissolved | Tunstall Healthcare (UK) Limited |
| NI069696 | 6 CLIFTONVILLE AVENUE MANAGEMENT COMPANY, dissolved | Stephen W Moore Limited |
| SC895484 | KULSHI ENTERPRISE LTD | Holyhead Marine Services Ltd |
| 00314578 | TAYLOR & FRANCIS LIMITED, dormant | Informa UK Limited t/a Taylor & Francis |
| 15700897 | VINCI FACILITIES LIMITED, dormant shell | VINCI Construction UK |
| SC332092 | BUSINESS STREAM LTD, non-trading | Scottish Water Business Stream (a sister company) |
| 04958135 | SCOTIA GAS NETWORKS LIMITED (the group parent) | Southern Gas Networks plc (a subsidiary) |
| 06099813 | GROUP 1 AUTOMOTIVE UK LIMITED (the parent) | Barons Automotive Limited |
| 03252690 | CONCUR TECHNOLOGIES (UK) LIMITED | Concur Holdings (Netherlands) BV |

Issue 448 holds these pairs out of the merge with `keep` verdicts (cohort `altid-2026-09-30`), so the PPON org, which
carries the RIGHT identity, is not folded into a wrong company number. That only stops the harm from growing. The
company-number org still stands with a number that points at someone else, and every API reader who joins on it gets
the wrong company.

## What to find out first

1. Size: the corpus holds many more GB company numbers than the 4,872 in the altid pairs. Counting them is a
   metadata read. Checking the register for each one is ~1.3 req/s against the public pages (`ch_fetch.py` in
   `.scratch/tender-db/448-campaign/`), or the Companies House REST API with a key (600 req/5 min).
2. Classes:
   - (a) no such number (404);
   - (b) dissolved or dormant, with a name unrelated to every name the org publishes;
   - (c) a live company whose name matches none of the org's names (another company's number);
   - (d) a parent or subsidiary of the named company.
   (a)–(c) are wrong numbers. (d) is a judgement.
3. Then the decision. The options are not exclusive:
   - withhold the identifier from matching (R2/E0/R3 and the altid arm never pair on it);
   - flag it on the API (`identifier_status: register_mismatch`);
   - move the org to a provisional identity keyed by name.

## Verify

A report or census counts GB company-number orgs by register class. Every (a)/(b)/(c) row is either withheld from
matching or flagged, and the ten numbers above read that way.

## 2026-09-30 08:5x UTC — sized

GB `national` org rows: 58,938. Of those, the company-number-shaped ones:

| shape | orgs |
|---|---|
| `GBCOH…` (minted from an FTS/TED `GB-COH-…` literal) | 23,805 |
| 8 digits | 7,402 |
| two letters + 6 digits (SC/NI/OC/SO/NC…) | 1,265 |

The rest are other shapes: 16,955 `GBPPON…`, and ~9.5k 13–14-digit and other literals (not company numbers; out of
scope here).

So ~32.5k orgs carry a company number. 4,872 distinct numbers are already checked
(`.scratch/tender-db/448-campaign/companies-house-2026-09-30.json`: 4 are 404s, and the class rows above come from
them).

**How to check the rest.** Not 32k page fetches against the public site (~7 h at a polite rate). Use Companies House's
free bulk product instead ("Free Company Data Product": a monthly CSV of every live company, with current and
previous names, one download of ~0.5 GB). A number missing from it is dissolved or never issued. The page fetch covers
only those, for the dissolved name. Offline plan:
1. download the snapshot to the box's `/data/archive/companies-house/`;
2. join it against the company-number orgs and their names (a read of `organizations` + `organization_names`: a
   snapshot job, not the serving DB);
3. classify (a)–(d) as above.

## 2026-09-30 09:5x UTC — census against the register snapshot (read-only)

Method: Companies House `BasicCompanyDataAsOneFile-2026-09-01.zip` (5,689,368 live companies, each with up to 10 previous
names). It was downloaded to the session scratchpad, not the box. The 58,938 GB `national` orgs were paged out of
/v1/sql by identifier (30 bounded pages, 2.8 s; `.scratch/tender-db/452-census/page_orgs.sh`). 30,857 carry a
company-number shape (`GBCOH…`, 8 digits, 2 letters + 6 digits), 30,844 distinct numbers. Each org's HEAD name was
compared with the register's current and previous names, using the 448 campaign's matcher.

| class | orgs |
|---|---|
| head matches the live register (current or previous name) | **28,545** (92.5%) |
| live company, head matches none of its names | 1,303 |
| number absent from the live register (dissolved, or never issued) | 1,009 |

The 1,303 split further:
- 819 share a distinctive token with a register name: trading names, brands, variants ("Maven Public Sector" / Aon UK);
- 125 are glued-word or accent variants (Hand2Hold / HAND 2 HOLD, Acumé / ACUME);
- **354** share nothing (`452-census/live-name-disjoint-candidates.json`). A read of samples finds real wrong numbers:
  - Cheltenham Borough Council under DIGIMUNE LTD;
  - Silver Energy Management under SPLENDIDO ESTATES;
  - Paragon Customer Communications under DJW COTTON CONSULTING;
  - Coastal Recycling under DEEP MOOR LF.

  It also finds parent-for-unit shapes (Pinehill hospital under Ramsay Health Care; HealthTrust Europe under HCA
  International) and a few trading names (thebigword under Link Up Mitaka).

The 1,009 absent numbers (`absent-from-live-register-2026-09-01.json`), from a register-page sample of 40:
- 17 dissolved with a matching name, i.e. a right number for a company that has since closed;
- **11 never issued** (404). Among them: typos (`0C415849` for OC415849), non-company numbers (a Dutch registration,
  an NHS trust code, `NP509120`), and garbage (`X338EBHC`);
- **8 dissolved under an unrelated name**;
- 4 open, converted or closed with a matching name.

**Estimate:** about 280 never-issued and about 200 dissolved-unrelated among the 1,009, plus most of the 354
live-disjoint. That is roughly **500–700 wrong company numbers across 30.9k orgs (~2%)**. The rest are right, or are
trading-name and parent/unit judgements.

Next unit: settle the ~350 live-disjoint and the ~480 absent-suspect exactly, with the 448 campaign's reviewer +
challenger shape (register pages for the absent ones). Then build the action (withhold from matching + API flag) as a
verdict table keyed by org and number, the way `org_merge_verdicts` is keyed by group.
