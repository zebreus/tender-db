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
