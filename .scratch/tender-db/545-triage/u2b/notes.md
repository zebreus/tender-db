# 545 units 2a-tail and 2b (2026-10-19)

Read-only. 10 of 12 `/v1/sql` requests were used (`requests.log`): b1-b7, r10_sec, r11_dest, r12_refs. There were no 408s and no retries, and 0 public GETs. Nothing was POSTed. No code, git, box write or admin job was touched.

Every read had its plan checked locally on this directory's plan.db (a copy of u2a's v551 plan.db with the two prod org indexes) by both planners: python sqlite3 (`planprobe.py`) and turso (`tplan.sh`). The plans are in `*.plan.txt`. All steps are SEARCH and none is a SCAN. `run.sh` is u2a's script repointed here, with the same refusals: no plan, a SCAN, or a 13th read.

Counts are given three ways: **rows** (spelling rows, one per (org, mention name, scheme)), **keys** (distinct (org, alt key)) and mentions.

## 2a-tail: HELD_K=6 on 2a's slices (no new slice reads)

- `HELD_K=6 candidates.py tail_k6 s1..s6` adds exactly **48 kept rows (42 keys)**: HELD-3 26, HELD-4 14, HELD-5 6, HELD-6 2.
- With K=2 the script reproduces u2a/cand.json byte for byte (`cmp`).
- Classes (`judge2b.py`, TAIL):

  | Class | Rows | Keys | Mentions |
  |---|---|---|---|
  | V | 15 | 12 | 44 |
  | T | 6 | 6 | 23 |
  | U | 11 | 9 | 31 |
  | A | 9 | 9 | 39 |
  | F? | 3 | 2 | 6 |
  | F | 4 | 4 | 12 |

  - The NHC rows are listed one by one, so "Stairlifts ... Framework 2026" comes out as T.
  - Luton and Dunstable Hospital (x3, on Bedfordshire Hospitals) is U: it is the predecessor trust and a site of the holder.
  - Pharmagraph (x3, on Acquisition Systems) is F?: both are small firms in the same niche, so a group relation is possible.
- Verdicts (`verdicts2b.py`):
  - **REHOME-HIGH: 2 rows, 2 keys, 6 mentions.**
    - Pullman Rail Ltd x3 on Transport for Wales → 31575835 GBCOH02820938 (tiebreak 3 mentions against 1).
    - HOSPITAL INNOVATIONS LIMITED x3 on Swansea Bay UHB → 30974714 GBCOH4261709 (tiebreak 12 against 1).
    - Both are supplier-side, and every notice carries a single role.
  - PARK-shared-procurement: 2 rows, 2 keys, 6 mentions. These are the buyer-side public-on-public cases: Wyre Forest DC x3 on Shropshire F&RS, and LB Hounslow x3 on LB Wandsworth.
  - PARK-agent-principal: 9 rows, 9 keys, 39 mentions.
  - PARK-sub-unit/group/sister: 11 rows, 9 keys, 31 mentions.
  - PARK-relation-unknown: 3 rows, 2 keys, 6 mentions.
  - NOT-545: 21 rows, 18 keys, 67 mentions.

## 2b: the GBCOH ranges (same scripts, K=6, same guards)

**Coverage: exact and complete in one session, so no 2c is needed.**
- Slices (`slices.json`):
  - b1 [GBCOH, GBCOH032)
  - b2 [GBCOH032, GBCOH04) and [GBCOH05, GBCOH066)
  - b3 [GBCOH066, GBCOH087)
  - b4 [GBCOH087, GBCOH111)
  - b5 [GBCOH111, GBCOH138)
  - b6 [GBCOH138, GBCOH344)
  - b7 [GBCOH344, GBCOI)
- Unit 1's q8 already covered [GBCOH04, GBCOH05).
- That gives 31,486 orgs, equal to q7's population for each of its 1,140 eight-character prefixes, with 0 differences and 0 orgs in two slices. 15,354 of them have 2 or more mentions.
- No slice was truncated. The slices returned 5,069 to 7,319 rows each and each took about 2.2-2.4 s wall time.

**Detection, K=6:**
- STRAY: 1,335 hits (302 kept).
- HELD-2: 343 (67 kept).
- HELD-3..6: 198 (41 kept; 108+51+12+27 — corrected from 200 by the adversarial check).
- **410 kept rows (396 keys); 1,466 dropped and not reviewed.**

**Classes** (`judge2b.py`, B):

| Class | Rows | Keys | Mentions |
|---|---|---|---|
| V | 225 | 215 | 290 |
| T | 20 | 20 | 24 |
| U | 63 | 60 | 92 |
| A | 3 | 3 | 9 |
| F? | 52 | 52 | 65 |
| F | 47 | 46 | 62 |

- Unit 1's six COH F all come back:
  - Bedford/Comensura → shared-section.
  - DDM, Diabetes UK and LUC → REHOME.
  - Autocross and London Cactus → no destination.
- "LITE Limited" x2 is V.
- Gleeds Cost Management x4 on College of Policing was re-classed F→A after r10 showed it as Procedure-Buyer on all 4 notices: a consultant running the buyer's procurements.

**r10** reads EVERY mention of each F key, on all notices, with its party and bid-party roles.
- In u2a, r7 read only the first and last notice. That is not enough for a key held 3-6 times.
- All 52 keys returned exactly their expected mention count.

**Verdicts (2b):**

| Verdict | Rows | Keys | Mentions |
|---|---|---|---|
| **REHOME-HIGH** | **22** | **22** | **28** |
| PARK-shared-section (G1) | 5 | 5 | 5 |
| PARK-shared-procurement (G2) | 3 | 3 | 5 |
| PARK-distinct-entities | 7 | 6 | 13 |
| PARK-no-destination | 7 | 7 | 8 |
| PARK-destination-ambiguous (hand refusal) | 2 | 2 | 2 |
| PARK-holder-number-on-destination (G4, new) | 1 | 1 | 1 |
| PARK-agent-principal | 3 | 3 | 9 |
| PARK-relation-unknown | 52 | 52 | 65 |
| PARK-sub-unit/group/sister | 63 | 60 | 92 |
| NOT-545 | 245 | 235 | 314 |

- PARK-shared-section (G1): Bedford/Comensura, Stockton/MRI, Newcastle/MGM, Haringey/Commonplace, and Kew/Fisher Scientific. In each, Procedure-Buyer and Tenderer bind the same section.
- PARK-shared-procurement (G2): Create PT on Howard PT, and Bolton Impact Trust and University of Essex on Embark MAT. All three are buyer-side.
- PARK-distinct-entities: Accenture x3 on Walnut; BMBC x2 and x1; UKCEH x2 on JNCC; CDDFT x1 and x3 on Synchronicity (one key); Ingeus.
- PARK-destination-ambiguous (hand refusal): PwC LLP and Hull CC. See guards.
- PARK-holder-number-on-destination (G4, new): Capita Resourcing on Conexia.

REHOME-HIGH 2b in detail:
- By destination rule: 7 identified-unique, 14 identified-tiebreak, and 1 provisional-exact (TPS Fruit & Veg → 21727460).
- By side: 20 supplier, 1 buyer (Lincolnshire Housing Partnership x4 on Mobysoft → 30917202 GBCOHRS007748), and 1 unbound (Ballicom: the move changes names only).

**Guards added in this unit (in verdicts2b.py; u2a/verdicts.py is reused unchanged as a module):**
- G1, side and unbound are computed over all of a key's notices (r10), not only the first and last.
- G2 gained 5 hand-listed (org, name) pairs. It is still a hand list, not a rule.
- G4 is new. If a destination-pool row carries the HOLDER's own company number, the name is the holder's former or trading name, and the row parks.
  - It fires once: the only identified "Capita Resourcing ... t/a Veredus" row (8881761) is keyed 3949686, which is Conexia's own number.
- REFUSE_DEST is a hand list. It covers the structural limit the 2a check found: near_identified cannot see an identified row whose head name is different.
  - PwC LLP → provisional "PWC LLP" is refused, because identified "PricewaterhouseCoopers LLP" rows exist (8881710 OC303525, 31609131 GBCOHOC309347, among others).
  - Hull CC → 31541038 is refused. It is keyed GBCOH168524345, a VAT-shaped number. The council's PPON row is 30914698 "Hull City Council".

**Flags for unit 3:**
- West Yorkshire CA is Tenderer on Bristol Temple Quarter LLP's number (OC451571). A public body on the supplier side is atypical, and a mis-keyed WYCA row 31565528 "GBCOHCO451571" exists.
- University of Liverpool x2 is Tenderer on Cardiff University, and RCPsych is Tenderer on Sussex. Bodies like these do appear as suppliers, but check them.
- The distinct-entities parks for Accenture and Ingeus trip on malformed variants of one number: 0475730 and 04757301EC3M3BD; 4320553. The register could settle them.

## Combined REHOME-HIGH
- 2a: 28 rows, 27 keys, 31 mentions.
- 2a-tail: 2 rows, 2 keys, 6 mentions.
- 2b: 22 rows, 22 keys, 28 mentions.
- **Total: 52 rows, 51 keys, 65 mentions.**

## Not covered (stated, not reviewed)
- Prefilter-dropped rows: 727 in 2a and 1,466 in 2b. Recall is inferred only from unit 1's labels.
- Tie-the-top keys (`ties.py` → `ties.json`): 2a has 121 keys, 12 of them prefilter-kept, including Bupa on 30926847. 2b has 134 keys, 16 kept, including "HYDRO INTERNATIONAL GROUP LTD" on M&N Electrical.
- Keys held more than 6 times below the top key (HELD_K=100000): 2a has 26 kept rows and 2b has 14. They are mostly V/U/A. Possible F: "Fujifilm Healthcare UK Limited" x7 on Mid and South Essex NHS FT. Possible U: Handicare on Savaria.
- GB-COH and GB-PPON mentions on orgs keyed some other way (bare numbers, VAT): this includes the 456 exhibit.
- q8 (COH04) has no last_notice column. This does not affect F rows, because r10 reads every notice.

## Files
- **Scripts:** gen_judge2b.py → judge2b.py; gen_r10.py, gen_r11.py, gen_r12.py; verdicts2b.py; ties.py.
- **Classes:** classes_tail.txt and classes_2b.txt (one line per kept row, with notes).
- **Outputs:**
  - tail_k6.json/txt and cand2b.json/txt (detector output);
  - tail_judged.json and cand2b_judged.json (classed rows);
  - verdicts2b.json and verdicts2b.txt (verdicts; the .txt carries per-candidate evidence: notice, section, roles, tenders, pool with refcounts, hints).
