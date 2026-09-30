# 452 — organizations carry company numbers that Companies House gives to another company, a dissolved shell, or no one

Status: done — DEPLOYED 2026-09-30 ~15:2x UTC (`e1dc081`, health green) and VERIFIED: 685 verdicts POSTed (cohort `452-census-2026-09-30`: recorded 685, stale 0, 473 served statuses changed = 371 wrong + 102 related); `/v1/organizations/16469211` (Harvey Nash, 02202746) serves `register_mismatch`, 13777708 `related_entity`, a `right` org null; the altid dry run (job 1718) withholds 30 owner rows. Follow-ups: issue 453 (re-key the 182 with a known right number), the 35 unclear/disputed.
Was status: ready-for-agent — ACTION UNIT BUILT 2026-09-30 ~15:xx UTC (gated, 142 suites): `org_identifier_verdicts`, POST `/admin/identifier-verdicts`, a live `wrong` number withheld from R2/E0/R3, the altid owners and the resolver's canonical bind, and `identifier_status` on `/v1/organizations`. NEXT: deploy, POST the 685 verdicts (`452-census/identifier-verdicts-2026-09-30.json`: 371 wrong, 102 related, 212 right), run the Verify.
Was status: ready-for-agent — CENSUS DONE 2026-09-30 13:0x UTC: 371 org identifiers are wrong company numbers (reviewer + challenger), 102 are a related company's, 182 of the wrong ones have the right number found (`452-census/verdicts-2026-09-30.json`). NEXT: the action unit, an identifier-verdict table that withholds a wrong number from matching and flags it on the API (design below).
Was status: ready-for-agent — filed 2026-09-30 from issue 448's review campaign. Next: size the class across every GB
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

    /root/aj.sh "/admin/case-reviews?table=identifier&cohort=452-census-2026-09-30&limit=5000" | python3 -c "import json,sys,collections; d=json.load(sys.stdin); print(len(d['rows']), dict(collections.Counter(r['verdict'] for r in d['rows'])))"
    curl -s https://tenders.zebreus.click/v1/organizations/16469211 | grep -o '"identifier_status":"[a-z_]*"'

- **done**: `685 {'wrong': 371, 'related': 102, 'right': 212}` (key order may differ) and `"identifier_status":"register_mismatch"`.
- **open**: fewer rows, or Harvey Nash's org without the flag.

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

## 2026-09-30 11:0x–13:0x UTC — the 720 cases settled (36 agents, reviewer + challenger)

Cases: the 354 live-name-disjoint, the 204 orgs whose number was never issued (404 on the register page), and the 162
whose number is a dissolved company with a name matching none of the org's names. The other 643 absent numbers are
dissolved companies with a MATCHING name: right numbers, since closed.
- Evidence per case: the org's head, its mention names with counts (/v1/sql), and the register entry (live snapshot or
  page).
- `rubric.md` and `cases-2026-09-30.json` are committed, with the verdicts in `verdicts-2026-09-30.json`. The challenger
  read every wrong-number and related-company verdict.

| settled | live-mismatch | never-issued | dissolved-mismatch | total |
|---|---|---|---|---|
| **wrong-number** (challenger agreed) | 77 | 204 | 90 | **371** |
| related-company (challenger agreed) | 77 | – | 25 | 102 |
| right-number (trading name, rename, spacing) | 175 | – | 37 | 212 |
| unclear, or disputed by the challenger | 25 | – | 10 | 35 |

- The reviewers found the right number for **182 of the 371** wrong ones. Most are one-digit typos or transpositions
  (Rolls-Royce under 01006142 for 01003142; Gasway 01458628 for 04158628) or another company's number (Fixatex under
  Elecheck).
- Related-company is mostly a subsidiary or sister company's number (Mitie Security under a Mitie business-services
  company; NFU Mutual under a dissolved NFU Mutual agency).

Over the 30,857 company-number orgs, **~1.2% carry a wrong number and ~0.3% a related company's**.

## Design for the action unit (next)

- `org_identifier_verdicts` (org_id, identifier, verdict `wrong|related|right`, correct_identifier, cohort, rationale,
  confidence), with POST `/admin/identifier-verdicts`. It mirrors `org_merge_verdicts`.
- The org keeps its identity row; what changes is what the number is TRUSTED for:
  - a `wrong` number is withheld from every identifier-based match (R2/E0/R3 keys, the altid arm, the resolver's
    exact-triple and canonical binds for NEW mentions of that literal) and flagged on the API
    (`identifier_status: "register_mismatch"`);
  - a `related` number is flagged, not withheld.
- A `correct_identifier` is recorded, never applied automatically. Re-keying an org is a merge-shaped change (the right
  number may already have its own org), so it goes through the merge arms with its own review.

## 2026-09-30 14:xx UTC — the action unit, built, reviewed, rebuilt (`aca3570`, `e1dc081`)

Built as designed above, then put through a 3-dimension adversarial review (12 agents; 9 findings, which verified down to 6 real
defects, all in the first commit). The shipped design, `e1dc081`:

- **Keyed by the identity triple, not the org id.** A verdict is stored under the (identifier, kind, country) the named
  org carries when it is POSTed; `org_id` stays as provenance. It applies to whichever org carries that exact triple.
  Review catch: a from-archive rebuild re-mints org ids from 1, and id-keyed verdicts would silently stop applying. The
  triple is what the resolver binds by, so the re-minted org carries it again. A verdict naming an org that no longer
  carries the number is skipped (`stale` in the POST answer). A re-keyed or merged-away org reads as unreviewed.
- **R2/E0/R3 and the altid arm leave a withheld org out of the preload** (member-scoped, like the consortium veto). The
  rest of its key group still merges, and every arm still has its name gate. Each summary counts `withheld`.
- **The resolver GUARDS the key instead of dropping the org.** The first cut left the withheld org out of the
  canonical map, and review showed two harms:
  - another spelling of the number (`GBCOH02202746` after `02202746`) minted an UNFLAGGED twin under the same wrong
    number, which the altid arm could then own;
  - a key the withheld org shared with the number's real owner stopped being poisoned, so the wrong publisher's
    mentions bound to that owner name-blind.

  Now a key with any withheld owner leaves the canonical map. Another spelling binds only to the one owner (withheld
  or not) whose names match the mention's names; otherwise it mints, and the mint joins the key's owners instead of
  claiming it. The anchor path and the altid alias find no owner there. **The exact triple still binds**: the
  byte-identical literal is the org's own published evidence. The fold log carries the counts (`[issue 452]` lines).
- **Change feed:** a POST that moves an org's served `identifier_status` appends `organization changed` for every org
  carrying the triple, then rings the cursor.
- **The first table above:** 8 of its 15 numbers are posted `wrong` (02202746, 02905600, 36389580, 11268329, 11386208,
  13322491, NI069696, SC895484). The other 7 (11747311, 00314578, 15700897, SC332092, 04958135, 06099813, 03252690)
  are RIGHT for their company-number org, whose head matches the register. There the fault was the FTS party pairing
  a related entity's PPON with that number, and issue 448's keep verdicts hold those pairs apart.
- The body is `452-census/identifier-verdicts-2026-09-30.json` (`verdict_post.py`): 371 wrong, 102 related, 212 right.
  All 685 orgs were live and carried the reviewed number (an id lookup at 14:0x UTC). Unclear and disputed verdicts (35)
  are not posted.

## 2026-09-30 ~15:2x UTC — deployed, posted, verified

- `e1dc081` deployed (health green, no warning lines). `POST /admin/identifier-verdicts` with the 685-verdict body answered
  `{"recorded":685,"stale":0,"changed":473}`: every reviewed org still carried its number, and 473 served statuses moved
  (the 371 wrong plus 102 related; the 212 right serve null, as before).
- Live: `/v1/organizations/16469211` (Harvey Nash Ltd, 02202746) → `"identifier_status":"register_mismatch"`;
  13777708 (William Cook Rail, 00053475) → `"related_entity"`; 11666834 (a `right`) → `null`.
- Altid dry run, job 1718, against 1716 before the verdicts:
  - "30 withheld by a wrong-number verdict" among the owners;
  - no company-number org 35 → 46, verdict-keep 29 → 21 (8 pairs a keep verdict held now have no trusted owner at all);
  - plan 4,604 → 4,603. One planned pair's company-number org carries a wrong number, so it is no longer merged.
- The resolver's `[issue 452]` lines appear at the next fold's resolver open.
