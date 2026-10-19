# 530 R2c: result (independent top-up of the post-Build-1b R2 re-read, dry plan job 2660)

Inputs:
- Issue 530, from "Unit 2 — decided" through "R2 re-read after Build 1b — PASSED" and its "Top-up (running)" line.
- `530-triage/decision.md`, amendments A5 to A8:
  - A5: the sample and the bound sentence;
  - A6: the city screen;
  - A7: role coherence;
  - A8: name-guard gaps.
- The plan report `530-triage/guid_twin_plan_2660.json` (Build 1b, deployed 8e9222c, dry job 2660).
- `530-triage/r2b/r2b-result.md`. R2b passed 60/60 R, but 17 of its picks were twin partners of R2 picks, because
  530002 XOR 530003 = 1. That left 43 independent picks.
- The picks `530-triage/r2c_picks.json`: 20 planned rows at salt 541337, none within 5 ids of the 120 rows R2 and R2b read.
- The stored mentions `r2c/q1.out.json`: a TED copy and a DÖE copy per row, 40 mentions.
- The reads `r2c/read_c1.md` (picks 0-9) and `r2c/read_c2.md` (picks 10-19).

## Verdict: the top-up PASSES (20 R, 0 N, 0 W; bar 0 W). R2's pass stands, and Build 2 may ship.

| list | picks | R | N | W | bar | result |
|---|---|---|---|---|---|---|
| top-up 20 (salt 541337, unstratified, independent) | 20 | 20 | 0 | 0 | 0 W | **pass** |
| buyer | 8 | 8 | 0 | 0 | | |
| review_or_info | 12 | 12 | 0 | 0 | | |
| other or tenderer | 0 | | | | | |

- **Stratum.** It follows the plan's convention for the row's section role: Procedure-Buyer is buyer, and every `Lot-` role,
  AddInfo and TenderReceipt included, is review_or_info. R2b classed its items 32, 33, 39 and 40 the same way. The draw itself
  is not stratified. Its 8 / 12 / 0 split is close to the planned shares: 24.8%, 73.3% and 1.8%.
- **No R depends on a mint.** All 19 distinct targets exist: each has identifier NULL, is provisional, is DE and carries
  exactly the row's name.
- **No W, so no fix.** Nothing goes into `GUID_TWIN_UNSAFE_NAMES` or `extra_unsafe_names`, and no guard changes.

## Neither of R2's W shapes occurs

- **No umbrella chamber name.** Each chamber target in the top-up names one body:
  - 622 "Vergabekammer des Bundes" and 3856 "Vergabekammern des Bundes": the federal chambers at the Bundeskartellamt, with
    numbered panels, one institution;
  - 3233 "Vergabekammer Westfalen": pinned safe in Build 1b;
  - 7615 "Vergabekammer Rheinland" and 7473 "… bei der Bezirksregierung Köln": the one Köln chamber;
  - 1168329: Brandenburg;
  - 18261510 "Vergabekammern des Landes MV bei dem Ministerium für WITA". The plural is MV's official designation. It has one
    seat (Johannes-Stelling-Str. 14, Schwerin), one e-mail and one phone. That is not R2's W1 shape, where "Vergabekammer NRW"
    pooled the Köln and Münster chambers, two seats with two e-mail domains.
- **No review slot prints another body.**
  - In all 20 groups, every section prints one street, one city and one e-mail. I re-extracted this mechanically from the 20 TED
    bodies.
  - No group mixes a buyer-side role with Lot-ReviewOrg, and none holds a Tenderer.
  - The groups fall into three kinds:
    - Buyer + AddInfo (+ TenderReceipt), in 12 groups;
    - Stadt Münster's own ReviewInfo beside its buyer triple, in 1 group (pick 3);
    - ReviewInfo + ReviewOrg (+ Mediator), in 7 groups (picks 0, 4, 5, 10, 15, 18 and 19).
  - Wherever a chamber is the ReviewOrg of a buyer-side notice, it is printed under its own name.
- **No stable key.** Each pick's GUID sits on one section, with the same value in both copies of one notice: 2 mentions. Its twin
  partners carry other fresh v4 values.

## Independence (checked, not assumed)

- **The draw.** I recomputed the smallest `((org_id XOR 541337) × 2654435761) mod 2³²` over `planned_row_ids`. The rule drops
  any id within 5 of the 120 rows R2 and R2b read, which excludes 391 planned rows. The result reproduces `r2c_picks.json`
  exactly. All 20 picks are planned.
- **Rows.** The nearest read row is 21 ids away (pick 3, 22824021, against R2's 22824042), and it is in another notice.
- **Notices.** None of the 40 pick notices appears in the R2 or R2b sources:
  - `r2/read_*.md` and `r2b/read_*.md`;
  - `r2-result.json` and `r2b-result.json`;
  - the `r2/bodies` and `r2b/bodies` directories.

  No two picks share a notice. **No pick shares a notice with a row R2 or R2b read, so all 20 are independent.**
- **Salt distance.** 541337 XOR 530002 = 21,707 and 541337 XOR 530003 = 21,706. Both are at least 256, as R2b advised. So a
  pick's XOR image sits at least 11,321 ids away, in another notice.
  - Two picks are the XOR image of a read row:
    - 22890922 (BLB Aachen) against the VBG pair 22879584/85;
    - 22904876 (VK Bund in a VBG notice) against 22926566 (LVwA Sachsen-Anhalt, R2b item 37).
  - That is a coupling of hash order only. The rows share no notice, group or target, so it is not a content dependence.
- **Targets.** There are 19 distinct targets: pick 1 and pick 11 both fold into 2443792, from different notices.
  - 12 had been read at target level: 622, 2443792, 2251109, 3233, 3856, 15234277, 1405749, 1222044, 7615, 1168329, 17012302 and
    1203308. Ten of them are R2 top-30 targets.
  - 7 are new: 1547849, 13200890, 4307071, 8437842, 1460530, 18261510 and 7473.
  - A draw over rows lands on the large targets in proportion to their rows. Each pick still tests a fresh fold, from its own
    notice into the target.
- **Correlated pair.** Picks 17 and 18 have the same buyer, Neubrandenburger Wohnungsgesellschaft. Their notices, groups and
  targets differ: 17 is the buyer's own slot and goes to 1460530; 18 is the MV chambers and goes to 18261510. I count both.

## The bound (A5's sentence, for the independent fresh picks after Build 1b)

All bounds are the one-sided 95% bound for 0 W in n: 1 − 0.05^(1/n). Rows are counted against the 10,995 planned rows.

| set | picks | W | bound | rows |
|---|---|---|---|---|
| R2c top-up alone | 20 | 0 | 13.9% | ~1,529 |
| **R2b independent 43 + R2c 20 (the headline)** | **63** | **0** | **4.6%** | **~511** |
| strict notice-level variant: also drop R2b items 26, 31, 44, 45 and 46 | 58 | 0 | 5.0% | ~553 |
| pooled with R2's 59 still-planned picks (indicative, not the bar) | 122 | 0 | 2.4% | ~267 |

- **Headline: 63 independent fresh picks after Build 1b, 0 W, 0 N.** The upper bound on the plan's error rate is 4.6%, about 511
  of 10,995 rows.
  - This is the first post-Build-1b count of independent picks to reach the size A5 asked of the random stratum, 60. R2b had 43
    independent picks.
  - One of the 63 is R only if Build 2 mints its target: R2b item 32, Boizenburg/KSM.
- **Strict variant.** R2b counted its items 26, 31, 44, 45 and 46 as independent. Each is a different group with a different
  target, but each sits in the same notice as an R2 pick. The top-up's rule ("shares a notice → not independent") would drop
  them, giving 58 picks and a bound of 5.0%. The conclusion is the same.
- **Pooled, indicative only.** R2's 59 still-planned picks were read on plan 2659, before Build 1b. 2660's planned set is a
  strict subset of 2659's, and all 59 scored R. Adding R2b's 43 and these 20 gives 122 distinct folds and 0 W, a bound of
  about 2.4%.
  - Four of the 122 are R only if their target is minted: R2's 23053387, 22856837 and 22895106 (all still planned) and R2b's
    item 32.
  - The pool is the union of three hash-ordered draws, not one simple random sample.

**By stratum, on the headline 63:**

| stratum | planned rows | independent picks (R2b + R2c) | bound | rows |
|---|---|---|---|---|
| buyer | 2,731 | 28 (20 + 8) | 10.1% | ~277 |
| review_or_info | 8,063 | 25 (13 + 12) | 11.3% | ~911 |
| other or tenderer | 201 | 10 (10 + 0) | 25.9% | ~52 |

## R verdicts with caveats (none affects the bar)

- **Target's own mentions not read.** The allowed endpoints have no route from an organization to its mentions. These verdicts
  rest on four things: the exact name, the name_prefix listing showing the target as the only identifier-NULL row of that name, a
  clean city screen, and the notice.
  - 1547849 RWTH Aachen University;
  - 13200890 JVA Castrop-Rauxel Zentralstelle;
  - 4307071 Stadt Ahlen;
  - 8437842 Samtgemeinde Bothel;
  - 7473 VK Rheinland bei der BezReg Köln.

  None of these names is generic or an umbrella.
- **Target read through a sibling planned notice.** 1547849 through 24332037; 1460530 and 18261510 through 24253163. Each sibling
  prints the same office in the same slots.
- **City screen.** `city.flagged` is complete (36 of 36). Of the 19 targets only 7615 is flagged, with ["50667", "koln"]:
  Köln's postcode typed into BT-513. For every other target, the newest-50 city set is either empty or exactly the planned city.
- **Template street.** Pick 5 prints the federal chambers at Bundeskanzlerplatz 2, but with the Bundeskartellamt's phone, fax
  and e-mail. This is VBG's notice template. R2 read_top2 item 21 saw the same buyer print Villemomblerstr. 76.
- **A reader claim corrected.** read_c2 says every target was re-read by `/v1/organizations/{id}`. In fact 4307071, 8437842 and
  7473 were confirmed from their rows in the name_prefix listing. Those rows carry the same fields: identifier NULL, provisional,
  DE, exact name and mention count. The verdicts do not change.
- **Asides, out of scope for 530.** Phone numbers have been taken as national identifiers:
  - 22695299 holds `T4942669831500`, the Bothel Samtgemeinde's phone;
  - 22318808, 22539480/81, 22575782, 22717048 and 22785935 hold Köln chamber numbers.

  None of them is a target or a planned row.

## Per-item verdicts

All 20 rows share these plan facts: DE; 2 mentions (the TED and DÖE copies of one notice, with the same section and raw
BT-501); planned whole; GUID still in place (identifier_kind national, not provisional); independent; no notice shared with
R2 or R2b. The reasons for each verdict are in `read_c1.md`, `read_c2.md` and `r2c-result.json` (`items[].why`).

| # | row | name | section / role | stratum | target (mentions) | verdict |
|---|---|---|---|---|---|---|
| 0 | 22815410 | Vergabekammer des Bundes | ORG-0003 Lot-ReviewInfo | review_or_info | 622 (111,305) | R |
| 1 | 22890922 | Bau- und Liegenschaftsbetrieb NRW Aachen | ORG-0005 Lot-TenderReceipt | review_or_info | 2443792 (573) | R |
| 2 | 22840477 | RWTH Aachen University | ORG-0001 Procedure-Buyer | buyer | 1547849 (469) | R |
| 3 | 22824021 | Stadt Münster | ORG-0001 Procedure-Buyer | buyer | 2251109 (192) | R |
| 4 | 22843623 | Vergabekammer Westfalen | ORG-0005 Lot-ReviewOrg | review_or_info | 3233 (37,696) | R |
| 5 | 22904876 | Vergabekammern des Bundes | ORG-0003 Lot-ReviewInfo | review_or_info | 3856 (17,642) | R |
| 6 | 22775107 | Bundesstadt Bonn - Referat Vergabedienste | ORG-0002 Lot-AddInfo | review_or_info | 15234277 (376) | R |
| 7 | 22949668 | JVA Castrop-Rauxel Zentralstelle für das Beschaffungswesen im Justizvollzug | ORG-0001 Procedure-Buyer | buyer | 13200890 (315) | R |
| 8 | 22874677 | HIL Heeresinstandsetzungslogistik GmbH | ORG-0001 Procedure-Buyer | buyer | 1405749 (2,076) | R |
| 9 | 22827487 | Bau- und Liegenschaftsbetrieb NRW Münster | ORG-0001 Procedure-Buyer | buyer | 1222044 (261) | R |
| 10 | 22917421 | Vergabekammer Rheinland | ORG-0002 Lot-ReviewInfo (twin ReviewOrg 22917422) | review_or_info | 7615 (29,617) | R |
| 11 | 22923417 | Bau- und Liegenschaftsbetrieb NRW Aachen | ORG-0002 Lot-AddInfo (twin Buyer 22923416) | review_or_info | 2443792 (573) | R |
| 12 | 22840850 | Stadt Ahlen | ORG-0001 Procedure-Buyer (twins AddInfo, TenderReceipt) | buyer | 4307071 (177) | R |
| 13 | 22845049 | Samtgemeinde Bothel | ORG-0003 Lot-AddInfo (twin Buyer 22845047) | review_or_info | 8437842 (9) | R |
| 14 | 22853395 | SBO Servicebetriebe Oberhausen - Eigenbetrieb der Stadt Oberhausen | ORG-0002 Lot-AddInfo (twin Buyer 22853394) | review_or_info | 17012302 (80) | R |
| 15 | 22997423 | Vergabekammer des Landes Brandenburg | ORG-0004 Lot-ReviewOrg (twin ReviewInfo 22997422) | review_or_info | 1168329 (10,598) | R |
| 16 | 22922428 | Bau- und Liegenschaftsbetrieb NRW Bielefeld | ORG-0001 Procedure-Buyer (twin AddInfo 22922429) | buyer | 1203308 (796) | R |
| 17 | 22782551 | Neubrandenburger Wohnungsgesellschaft mbH | ORG-0001 Procedure-Buyer (twin AddInfo 22782552) | buyer | 1460530 (186) | R |
| 18 | 22753860 | Vergabekammern des Landes Mecklenburg-Vorpommern bei dem Ministerium für Wirtschaft, Infrastruktur, Tourismus und Arbeit | ORG-0003 Lot-ReviewInfo (twin ReviewOrg 22753861) | review_or_info | 18261510 (620) | R |
| 19 | 22938395 | Vergabekammer Rheinland bei der Bezirksregierung Köln | ORG-0003 Lot-ReviewOrg (twin ReviewInfo 22938394) | review_or_info | 7473 (6,101) | R |

## What this tally re-checked (0 GETs)

- All 20 picks are in `planned_row_ids`, and the draw reproduces from the plan.
- In `q1.out.json`, each pick has 2 mentions with the same section, name and raw BT-501 in both copies. In each TED body, the
  pick's section carries that raw BT-501.
- From the 20 TED bodies (`r2c/bodies/`), each pick's twin group: sections, OPT-300/301 roles, street, city and e-mail. Every
  group is single-address and role-coherent, as listed above.
- The 19 targets, from their organization bodies or name_prefix rows: identifier NULL, provisional, DE and the exact name, with
  mention counts as the reads give them.
- The name_prefix listings show no other identifier-NULL row with the exact name for the five targets found that way.
- The plan's lists: `city.flagged` (only 7615) and `r2_top_targets` (10 of the targets).
- The independence checks above.
- The GET log, re-counted below.

## Evidence base

- **GETs: 86 public GETs** are logged in `r2c/api_requests.log`, from 21:18:48.862Z to 21:25:09.464Z.
  - By endpoint: 42 `/v1/notices/{id}/content`, 36 `/v1/organizations/{id}` and 8 `/v1/organizations?name_prefix=…&country=DE`
    (one of them a cursor page 2).
  - By reader: c1 42 and c2 44.
  - All 86 returned 200. There was no 408 and no retry (every line is `attempt=1`), and no path was fetched twice.
  - The smallest gap between any two logged requests is 2.05 s. The readers serialized through `r2c/.api.lock` and `.api.last`.
  - 86 bodies are saved in `r2c/bodies/`. The readers also reused bodies already on disk from `r2/bodies` and `r2b/bodies`:
    47408004, 47414536 and the target-notice bodies listed in read_c1, plus 24331367.
- **The stored mentions in `q1.out.json`** were an input, one bounded read logged in `r2c/requests.log` at 21:16:31Z. This step
  did not make that read.
- **This tally made 0 GETs.** No `/v1/sql`, ssh, POST, git or cargo was used.
- **Machine-readable copy:** `r2c/r2c-result.json`.

## Next

- **R2 stays passed, now on 63 independent fresh picks (0 W).** Build 2 may ship as decided:
  - the KEEP file;
  - the gate wired in;
  - the wet path;
  - the recurrence arm (A1);
  - the mint of the 52 missing keys (312 rows, 636 mentions).
- **R3** follows the wet run.
- **Any later draw** (R3, the recurrence arm) should exclude the 140 rows already read and their notices. It should also use a
  salt whose XOR with 530002, 530003 and 541337 is at least 256.
