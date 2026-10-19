# 572 unit 1 — measure: a buying agent's identifier and the bodies it buys for (2026-10-19)

Read-only. 7 `/v1/sql` reads (budget 60), 35 public API GETs (budget 150), 13 GETs of Find a Tender's own
published OCDS packages (public, OGL; the source, not the box), 0 non-200, 0 408, no retries. Every join was
plan-probed locally first (python sqlite3 and turso `plan-probe` on 545 u2b's `plan.db`, which carries the prod
schema and the two prod org indexes); every step is a SEARCH, none a SCAN (`q*.plan.txt`). Every request is
in `requests.log`. No repo file outside this directory was touched; no git, no admin job.

## Summary

- **Size.** 545's judged candidates hold **65 class-A rows, 64 keys, 129 mentions** (GBPPON 62 / 61 / 120,
  GBCOH 3 / 3 / 9). Read exhaustively for the same 15 agents (no K cap, no prefilter, all 45,824 GBPPON/GBCOH-keyed
  orgs), the class is **191 mentions, 85 keys, 190 notices**. 545's detectors saw 129 of the 191.
- **Direction.** 153 of the 191 mentions (70 keys) print the **principal's name under the agent's identifier**
  ("P@A"). Northern Housing Consortium alone accounts for 97 mentions (52 member bodies), NWSSP for 41. The other
  38 (15 keys) print the **agent's name under a principal's identifier** ("A@P"). 33 of those 38 sit on the
  actual buyer's own identifier, so only the printed name is wrong (3 read, 30 presumed from the same
  agent/holder pairs). 3 sit on the agent's host council: Star on
  Trafford, Ardal on Cardiff. 2 sit on an unrelated body: Pagabo on CNTW's PPON.
- **Every one is an FTS notice** (`q7_sources`: 190 of 190 notices are `fts:ocds-1.1`), **and every one binds as
  `Procedure-Buyer`**. The exceptions are 2 Tenderer slips.
- **The source carries the agent role sometimes; tender-db never does.**
  - In the source: Find a Tender publishes the agent as a separate `procuringEntity` party, with a
    `roleDetails` "on behalf of" sentence, on **7 of the 13** exemplar notices. The other 6 have a buyer party
    only: NHC ×3, Welland, TMT and Gibbs Laidler.
  - In tender-db: the FTS parser maps both `buyer` and `procuringEntity` to `OPT-300-Procedure-Buyer`
    (`crates/ingest/src/fts/parse.rs:249`). It drops `roleDetails` and `details.classifications` (UK_CA_TYPE,
    the BT-11 analogue).
  - Class-wide, **143 of the 191 mentions (75%)** sit on a notice with a single buyer section. There the agent
    and the principal are fused into one party, so no role field could separate them. This covers NHC 97/97 and
    TMT 11/11.
- **Served today** (issue 456 design 3): every class mention is served as `organization_id` /
  `organization_name` = the identifier holder, with the printed name as `mention_name`.
  - Example: tender 8641682's buyer reads `[30953692, "Northern Housing Consortium Ltd", "Birmingham City Council"]`,
    while Birmingham has its own org, 30914707 (GBPPONPCJJ4826QMLZ).
  - 110 of the 191 party rows are on their tender's current version: P@A 97 of 153, A@P 13 of 38.

## (a) Size

### Part 1: 545's judged candidates (detector-bound)

Sources:
- `u2a/cand_judged.json`: 52 A rows.
- The unit-1 carried Pagabo row in `u2a/verdicts.json`.
- `u2b/tail_judged.json`: 9 A rows.
- `u2b/cand2b_judged.json`: 3 A rows.

Rows are 545's spelling rows. Keys are distinct (org, `keys.alt`). The script is `measure_a.py`; its outputs are
`a_rows.json` and `a_stats.txt`.

| slice | rows | keys | mentions | orgs |
|---|---|---|---|---|
| **total** | **65** | **64** | **129** | 21 |
| u2a (GBPPON, K=2) | 52 | 51 | 79 | 15 |
| 2a-tail (GBPPON, K 3..6) | 9 | 9 | 39 | 4 |
| unit-1 carried (Pagabo) | 1 | 1 | 2 | 1 |
| 2b (GBCOH, K=6) | 3 | 3 | 9 | 2 |
| P@A (principal's name, agent's identifier) | 51 | 51 | 99 | 8 |
| A@P (agent's name, principal's identifier) | 14 | 13 | 30 | 13 |

572's header "52 kept rows / 53 keys / 79 mentions" is u2a alone. 52 rows are 51 keys, because Star's two
spellings share one key. "53" is 545's verdict count, which adds the carried Pagabo row.

By agent, part 1: NHC 41/41/81 · NWSSP 6/6/13 (A@P 5/5/7, P@A HEIW 1/1/6) · TMT 2/2/11 · Gibbs Laidler 2/2/5 ·
Gleeds 1/1/4 · Ginger 2/2/2 · Invictus 2/2/2 · Pagabo 1/1/2 · Scape 1/1/2 · Star 2/1/2 · Ardal 1/1/1 ·
NHS London Commercial Hub 1/1/1 · BSO PaLS 1/1/1 · RPJ3 1/1/1 · Welland 1/1/1.

### Part 2: exhaustive within the slices, same 15 agents

Method:
- Read every name on each agent's own org(s), and every agent-named mention on any other org.
- Use all 545 slice reads: u2a s1–s6 and unit 1's q3 cover all 14,338 GBPPON orgs; u2b b1–b7 and q8 cover all
  31,486 GBCOH orgs.
- Excluded:
  - the agent's own spelling variants;
  - NWSSP's legal host, Velindre (×47 on NWSSP's PPON, U: Velindre ×46 and Welsh Blood Service ×1);
  - one framework title on NHC's PPON ("Stairlifts and Lifting Equipment Framework 2026" ×4, T);
  - namesake companies (Welland House Care Centre, TMT First, PALS Ltd, other Invictus/Gingerbread companies).
- Re-read live with role references in `q4_class_refs`; the totals match the slices exactly.
- Scripts: `measure_a.py` part 2, `analyze_q4.py`.

| agent | kind | direction | mentions | keys | orgs |
|---|---|---|---|---|---|
| Northern Housing Consortium | housing consortium | P@A | **97** | **52** | 1 |
| NWSSP (Procurement Services) | NHS Wales shared service | P@A | 41 | 7 | 1 |
| NWSSP | | A@P | 15 | 7 | 6 |
| TMT Procurement Partnership | Kent councils' shared service | A@P | 11 | 2 | 2 |
| Gibbs Laidler Consulting | consultant for housing associations | P@A | 8 | 4 | 1 |
| Gleeds Cost Management | consultant as co-buyer | A@P | 4 | 1 | 1 |
| STAR Procurement | GM councils' shared service | A@P | 2 | 1 | 1 |
| Invictus Partnership | schools broker | P@A | 2 | 2 | 1 |
| Ginger Energy | energy broker | P@A | 2 | 2 | 1 |
| SCAPE | framework operator | A@P | 2 | 1 | 1 |
| Pagabo | framework operator | A@P | 2 | 1 | 1 |
| Ardal | Cardiff-run shared service | A@P | 1 | 1 | 1 |
| Welland Procurement | Rutland et al. shared service | P@A | 1 | 1 | 1 |
| RPJ3 Group | schools broker | P@A | 1 | 1 | 1 |
| NHS London Commercial Hub | London ICBs' hub | P@A | 1 | 1 | 1 |
| BSO PaLS | NI HSC shared service | A@P | 1 | 1 | 1 |
| **total** | | P@A 153 / 70 · A@P 38 / 15 | **191** | **85** | 190 notices |

What 545's detectors did not see:
- **NHC:** 11 more member bodies, 16 mentions. The prefilter dropped them for sharing the token "housing" with
  "Northern Housing Consortium": Solihull Community Housing ×5, LIVIN Housing ×2, Arches, Believe, Blackpool
  Coastal, Housing 21, Islington & Shoreditch HA, London & Quadrant, One Vision, South Lakes, Pioneer.
- **NWSSP P@A:** 35 mentions beyond HEIW ×6: Hywel Dda ×12, Cardiff and Vale ×10, Public Health Wales NHS Trust
  ×9, Betsi ×2, ABUHB ×1, NWJCC ×1.
- **NWSSP A@P:** Cardiff and Vale UHB ×7, held 7 times, above the K=6 cap.
- **Gibbs Laidler:** Hull Churches HA ×2 and Newlon ×1.

Nothing else agent-shaped turned up:
- A broad pattern scan of the slices (consortium / procurement / purchasing / shared services / "on behalf of" /
  ESPO / YPO / CCS / LHC / Fusion21 / NEPO / SUPC …) found no further agent with principal names on its org,
  beyond the 15 agents above.
- It found no agent name on another org beyond them either; the hits were the holder's own variants or hosted
  bodies.
- Agents keyed some other way (bare numbers, VAT, provisional) are outside the slices and not measured.

### A@P split by who the identifier belongs to

| subtype | mentions | rows | what the API serves |
|---|---|---|---|
| own buyer | 33 | NWSSP on the health board / PHW / Swansea Univ. PPONs 15, TMT on Maidstone 5 + Tunbridge Wells 6, Gleeds on College of Policing 4, Scape on Arc 2, PaLS on Northern HSCT 1 | the right buyer org, with the agent's name as `mention_name` (E4, E10, E11 read; the rest presumed from the same pairs) |
| host council | 3 | Star Procurement on Trafford's PPON buying for Rochdale ×2; Ardal on Cardiff's PPON buying for WLGA ×1 | **the host is served as a buyer of another body's procurement** (E6, E7) |
| unrelated | 2 | Pagabo on Cumbria, Northumberland, Tyne and Wear NHS FT's PPON, buying for The Education Alliance | CNTW served as buyer: 545-F-shaped |

Caveats:
- NWSSP notice 46819472 crosses two names over two PPONs: ABUHB's name sits on NWSSP's PPON and NWSSP's name on
  ABUHB's.
- On Gleeds' 4 notices, College of Policing is also printed in its own section, so CoP is served twice as
  Procedure-Buyer.

### A-shaped rows 545 labelled otherwise

Both are shown from 545's files, and **not counted** in the totals above.

| rows | 545 label | evidence |
|---|---|---|
| University of Essex ×2 + The Bolton Impact Trust ×2 on Embark MAT 31577532 (GBCOH11717748) | F, PARK-shared-procurement | `545-triage/u3/guards.md` (b): notice 46851503 is Essex's Energy Management dynamic market, with Dukefield Energy as co-buyer "(the agent)". Verdict "stay PARKED (A or shared procurement)", and first a possible altid defect: PYLY-4964-TXGH may be Essex's or Dukefield's PPON on Embark's org. |
| Create Partnership Trust ×1 on The Howard Partnership Trust 30917637 (GBCOH07597068) | F, PARK-shared-procurement | `guards.md` (b): notice 46758153 prints Create PT with the holder's own number, and broker Ginger Energy (an A agent) as second Procedure-Buyer. Verdict "V-or-A". |

The U label on "Laser Energy" on Kent CC (u2a) is the same boundary as Star/Trafford and Ardal/Cardiff: a buying
service with no legal personality of its own, keyed by its host council. 545 called LASER U and Star/Ardal A. Only
Star and Ardal buy for other members on the host's identifier (E6, E7).

## (b) Exemplars: how the relation is published

There are 13 notices, one or more per top agent:
- For each: `/v1/notices/{id}/content`, the tender detail, and the bound org from `q1_exemplars`.
- E11's content came from 545's cached body, so no GET was spent.
- Each exemplar's FTS release package came from find-tender.service.gov.uk.
- Bodies are in `bodies/` and `fts/`. `show_content.py` and `show_fts.py` print them. `measure.json` has the
  full structured record.

| ex | notice / FTS id / type | tender (ver) | ORG sections as printed → bound org | FTS source roles (UK_CA_TYPE; roleDetails) | served `Procedure-Buyer` today (org, head ← mention_name) |
|---|---|---|---|---|---|
| E1 NHC | 46793765 / 018561-2026 / UK6 | 8641682 (v1 of 2) | "Birmingham City Council" + **NHC's PPON** PJXZ-4423-NDBT → 30953692 NHC; Ecclesiastical (Tenderer) | buyer only: PJXZ "Birmingham City Council" (publicAuthoritySubCentralGovernment); text: "made under the Northern Housing Consortium's …" | 30953692 Northern Housing Consortium Ltd ← "Birmingham City Council" |
| E2 NHC | 46789522 / 014218-2026 / UK6 | 8636304 (v1 of 2) | "Darlington Borough Council" + PJXZ → NHC | buyer only (same shape) | 30953692 NHC ← "Darlington Borough Council" |
| E3 NHC | 46794092 / 018898-2026 / UK7 | 8677647 (v1) | "Karbon Homes (54North Homes)" + PJXZ → NHC | buyer only; details.url karbonhomes.co.uk | 30953692 NHC ← "Karbon Homes (54North Homes)" |
| E4 NWSSP | 46816840 / 042613-2026 / UK4 | 8654651 (v1 of 2) | NWSSP's name + **ABUHB's PPON** PVLN-4733-WJBQ → 30919377 ABUHB; NWSSP + own PPON PXWV → 30913738 | buyer = PVLN with NWSSP's name; **procuringEntity** = PXWV NWSSP: "NHS Shared Services Local Procurement is acting on behalf of Aneurin Bevan University Health Board" | current v2: 30913738 NWSSP ← NWSSP; 30919377 ABUHB ← ABUHB (v1's mention is history) |
| E5 NWSSP | 46787268 / 011922-2026 / UK4 | 8698064 (v1 of 4) | "HEIW" + **NWSSP's PPON** PXWV → 30913738 NWSSP; NWSSP's name + GB-NHS-RQF (Velindre's ODS code) → 15641552 (provisional "NWSSP") | buyer = PXWV "HEIW" (central govt, GB-WLS); **procuringEntity** = GB-NHS-RQF NWSSP: "… on behalf of HEIW …" | 15641552 NWSSP ← NWSSP; 30913738 NWSSP ← "HEIW" (HEIW's own org 30917293 absent) |
| E6 Star | 46817944 / 043745-2026 / UK4 | 8655231 (v1) | "Star Procurement" + **Trafford's PPON** PGZW-1429-QQZQ → 30917275 Trafford; Rochdale + own PPON → 30916230 | buyer = Rochdale; **procuringEntity** = PGZW "Star Procurement": "Managing the Tender" | 30916230 Rochdale; **30917275 Trafford Council ← "Star Procurement"** |
| E7 Ardal | 46765017 / 076139-2025 / UK2 | 8621643 (v1 of 5) | "Ardal" + **Cardiff's PPON** PVHP-5769-YHRQ → 30913763 Cardiff; WLGA + own PPON → 30928156 | buyer = WLGA; **procuringEntity** = PVHP "Ardal": "Ardal is the new name for the procurement service run by Cardiff Council …" | current v5: **30913763 Cardiff Council ← "Cardiff Council"**; 30928156 WLGA |
| E8 Welland | 46845134 / 071214-2026 / UK1 | 8677040 (v1) | "Rutland County Council" + **Welland's PPON** PVMQ-1845-NYPP → 30913798 Welland; Leicestershire, Leicester with own PPONs | three buyers, no procuringEntity | **30913798 Welland Procurement ← "Rutland County Council"** (Rutland's own org 30915335 absent) |
| E9 Ginger | 46737636 / 048614-2025 / UK6 | 8639202 (v1) | "The Howard Partnership Trust" + PLMN-9829-JXLV → 30968739 (head "Ginger energy"); Ginger Energy + PJYB → 31540891 | buyer = PLMN "The Howard Partnership Trust" (publicUndertaking); **procuringEntity** = PJYB Ginger: "managed on behalf of THPT by Ginger Energy Limited" | 30968739 Ginger energy ← "The Howard Partnership Trust"; 31540891 Ginger Energy |
| E10 TMT | 46750905 / 061940-2025 / UK4 | 8613154 (v1 of 4) | "TMT Procurement Partnership" + **Maidstone's PPON** PGRT-5366-CGWY → 30916821 Maidstone (only buyer) | buyer only, named TMT; text "Maidstone Borough Council (MBC) is looking to appoint …" | current v4: 30916821 Maidstone ← "Maidstone Borough Council" |
| E11 Gleeds | 46693105 / 089087-2026 / UK4 | 8580124 (v1 of 4) | CoP + COH 08235199 + PPON PJNL → 30914436; "GLEEDS COST MANAGEMENT LIMITED" + **CoP's PPON** PJNL-2763-PVHM → 30914436 | buyer = CoP; **procuringEntity** = PJNL Gleeds: "Specification preparation/ evaluation" (details.url college.police.uk) | 30914436 CoP ← CoP; **30914436 CoP ← "GLEEDS COST MANAGEMENT LIMITED"** (CoP twice) |
| E12 Gibbs | 46757368 / 068424-2025 / UK6 | 8596944 (v2) | "English Rural Housing Association" + PGJB-3773-RCMH → 30913722 Gibbs Laidler (PGJB merged into its COH org); Marsh (Tenderer) | buyer only; Gibbs not printed at all | **30913722 GIBBS LAIDLER CONSULTING LLP ← "English Rural Housing Association"** |
| E13 RPJ3 | 46869370 / 009438-2025 / UK4 | 8687157 (v1) | "Remarkable Autism" + PRYY-6263-NQHG → 30914820 (head "RPJ3 Group Limited on Behalf of Aspirational Futures MAT"); RPJ3 + COH 04642430 → 31594875 | buyer = PRYY "Remarkable Autism"; **procuringEntity** = RPJ3 COH: "RPJ3 Group will manage the procurement process on behalf of Remarkable Autism" | 30914820 RPJ3 … ← "Remarkable Autism"; 31594875 RPJ3 |

What the exemplars show:
1. **The mechanism is the publishing account's PPON.**
   - In the P@A shape, FTS's `buyer` party carries the agent's PPON (the account that published) under the
     principal's typed name. Its `details` often describe the principal: the url is the principal's site in E3,
     E9 and E13, though NWSSP's in E5. UK_CA_TYPE is filled for the buyer party only.
   - Whether the agent appears too, as `procuringEntity`, depends on the publisher. RPJ3, Ginger and NWSSP add
     it. NHC, Welland and Gibbs do not.
2. **In the A@P shape, the agent sits in the `procuringEntity` slot, keyed by a principal or host PPON.**
   - Examples: Star with Trafford's PPON, Ardal with Cardiff's, Gleeds with CoP's.
   - Alternatively it sits in the `buyer` slot under its own name. Examples: TMT on Maidstone's PPON, and NWSSP
     on ABUHB's in E4.
3. **No eForms-style field carries the relation.**
   - There is no BT-11 equivalent: UK_CA_TYPE sits on `buyer` parties only and is dropped. There is no
     Procedure-SProvider (FTS's own `procuringEntity` folds into Procedure-Buyer), and no on-behalf-of field
     except the free-text `roleDetails`, which is dropped.
   - The only on-behalf text tender-db keeps is BT-24 prose (NHC's "made under the Northern Housing Consortium's
     …", TMT's "Maidstone Borough Council (MBC) is looking …").
4. **Principals usually have their own org.** Each name below is followed by its own org row (from
   `q5_principal_orgs`):
   - Birmingham: 30914707 GBPPONPCJJ4826QMLZ.
   - Darlington: 30914393 GBPPONPCRZ3744WXXZ.
   - Karbon: 30915057 GBPPONPCDL6326LXPW, plus CHC/MPR rows.
   - HEIW: 30917293 GBPPONPRNH7564JLCQ, plus 31539418 GBNHST1530.
   - Rutland: 30915335 GBPPONPMQJ3429NZWQ.
   - Howard PT: 30917637 GBCOH07597068.
   - ABUHB: 30919377.
   - Hywel Dda: 30926960.
   - Solihull Community Housing: 5539040 '04462630'.
   - The Education Alliance: 31578022 / 31587300.
   - Without one: English Rural HA (provisional 2291108 only) and Remarkable Autism (none).

## Role carried: counts

| where | carries the agent role distinctly? | count |
|---|---|---|
| FTS source, exemplar notices | yes: a `procuringEntity` party, plus `roleDetails` on all 7 | **7 of 13** (E4, E5, E6, E7, E9, E11, E13); buyer only on 6 (E1–E3, E8, E10, E12) |
| tender-db, exemplar notices | **no**: both parties are `Procedure-Buyer`; `roleDetails` and `details` are not stored | 0 of 13 |
| tender-db, whole class (`q2_roles`, `q4_class_refs`) | 189 of 191 A mentions bind as `Procedure-Buyer`, 2 as Tenderer (Riverside on NHC's PPON, Hywel Dda on NWSSP's) | — |
| proxy: buyer sections on the A mention's notice | a single buyer section means the source had no separate agent/principal party | **143 of 191 single** (NHC 97/97, TMT 11/11, NWSSP 23/56, Gibbs 6/8, Scape 2, Invictus 2, PaLS 1, NHSLCH 1); 48 have 2 or more (agent and principal both printed, or co-buyers) |

## Served today: class-wide

`q6_current` reads, for each class party row, whether it is on the tender's current version. That version is what
`/v1/tenders/{id}` `parties[]` and `v_tender_buyers` show.

| direction | on current version | older version only |
|---|---|---|
| P@A | 97 | 56 |
| A@P | 13 | 25 |

By agent:
- NHC 59 / 38.
- NWSSP P@A 26 / 15.
- NWSSP A@P 6 / 9.
- TMT 1 / 10.
- Gibbs 5 / 3.
- Star 2 / 0.
- Pagabo 2 / 0.

Each current P@A row credits the agent org as buyer (`?buyer=` by org id, buyer statistics). The principal shows
only as `mention_name`, the 456 design 3 read.

## (c) Per-agent org: names and counts

These were read twice:
- in the slices, a snapshot taken earlier on 2026-10-19;
- live, through `q2_roles` (per name and role, via party rows).

The totals agree.

| org | identifier | mentions | agent's own name | principal names (count) |
|---|---|---|---|---|
| 30953692 Northern Housing Consortium Ltd | GBPPONPJXZ4423NDBT | 124 | 23 | **97 over 52 bodies**: Darlington 5, Solihull Community Housing 5, Rochdale 4, Stockton-on-Tees 4, East Suffolk 3, Newcastle 3, North Northamptonshire 3; 24 councils and LIVIN Housing at 2 each (Birmingham … Wigan); 20 bodies at 1 each (Arches, Believe, Bernicia, Blackpool Coastal, Gateshead, Housing 21, Islington & Shoreditch, Karbon/54North, L&Q, Moat, One Manchester, One Vision, Onward, Orbit, Ongo, Riverside, St Leger, South Lakes, Guinness, Pioneer). Plus the framework title ×4 |
| 30913738 NWSSP-Procurement Services | GBPPONPXWV6492CGMN | 1,170 | 1,048 (+32 own variants the pattern misses, including "NHS Wales" ×10; "NHS" ×2) | 41: Hywel Dda 12, Cardiff and Vale 10, Public Health Wales NHS Trust 9, HEIW 6, Betsi 2, ABUHB 1, NWJCC 1. Host: Velindre ×46, Welsh Blood Service ×1 |
| 30913798 Welland Procurement | GBPPONPVMQ1845NYPP | 287 | 286 | Rutland County Council 1 |
| 30913722 Gibbs Laidler Consulting LLP | GBCOHOC322760 (+ merged GBPPONPGJB3773RCMH) | 163 | 155 | English Rural HA 3, Hull Churches HA 2, YMCA Saint Pauls 2, Newlon 1 |
| 30930308 Invictus Partnership | GBPPONPGGP8622HHRX | 56 | 54 | CORE Education Trust 1, William Farr C of E School 1 |
| 30968739 "Ginger energy" | GBPPONPLMN9829JXLV | 31 | 29 | The Howard Partnership Trust 1, The Keys Academy Trust 1 |
| 30916628 NHS London Commercial Hub | GBPPONPBWD3998QHRQ | 26 | 25 | NHS North Central London ICB 1 |
| 30914820 "RPJ3 Group Limited on Behalf of Aspirational Futures MAT" | GBPPONPRYY6263NQHG | 24 | 23 (4 carry "on behalf of <trust>" in the name) | Remarkable Autism 1 |

Agents without an identified PPON or COH row of their own, from the `q3_agent_orgs` name_norm seeks:
- **STAR Procurement:** 24 provisional rows, e.g. "STAR Procurement - Trafford Council", "STAR Procurement / Rochdale
  Council".
- **Ardal:** none in GB.
- **TMT:** none.
- **Scape:** 11842492 '05660357', a bare number.

Several provisional rows print the relation in the name itself:
- "Northern Housing Consortium Ltd (on behalf of all current and future Members)";
- "Welland Procurement on Behalf of Bassetlaw District Council", "Welland Procurement pp East Northants Council";
- "NHS London Commercial Hub on behalf of NHS SEL ICB".

## Side findings (not 572 work)

1. **The FTS checklist marks `parties[].roleDetails` Mapped although nothing reads it.**
   - The `("parties", Mapped(...))` container entry covers the whole subtree (`checklist.rs:48`, longest match), but
     the `Party` struct has no `roleDetails`.
   - Also, `parties[].roles` doc-lists its targets without saying that `procuringEntity` becomes
     Procedure-Buyer.
   - This is a silent drop that ADR-0004's census cannot see.
2. **NWSSP is split across several rows:**
   - 30913738 (PPON, 1,170 mentions);
   - **15641552** (provisional, 2,123 mentions, which takes the `ORG-GB-NHS-RQF` sections; RQF is Velindre's ODS
     code);
   - 31537662 (GBNHS7A7);
   - 114 provisional spellings.

   This belongs to the 513/519 NHS-body work.
3. **Gleeds Cost Management is split** between 11550220 '06472932' and 31615833 'GBCOH6472932' (leading zero,
   470/473).
4. **Pagabo on CNTW's PPON, buying for The Education Alliance** (46844727, 46844778), is 545-F-shaped rather than
   agent-on-host. It may belong in a 545 follow-up cohort.

## Files (all in this directory)

| file | what it is |
|---|---|
| `measure.json` | the structured result |
| `measure_a.py` | part 1 and part 2 sizing |
| `a_rows.json`, `a_exhaustive.json`, `a_stats.txt` | sizing outputs |
| `q1_exemplars.sql`, `q2_roles.sql`, `q3_agent_orgs.sql`, `q4_class_refs.sql`, `q5_principal_orgs.sql`, `q6_current.sql`, `q7_sources.sql` | the 7 reads, each with `.plan.txt` and `.out.json` |
| `analyze_q4.py`, `analyze_q6.py` | analysis, with their `.summary.json` outputs |
| `q2_roles.txt` | readable form of `q2_roles` |
| `build_measure.py` | assembles `measure.json` |
| `run.sh`, `get.sh`, `ftsget.sh`, `planprobe.py`, `tplan.sh` | runners, with hard caps and logging |
| `show_content.py`, `show_fts.py` | print the cached bodies |
| `bodies/` | public API bodies |
| `fts/` | FTS release packages |
| `requests.log` | every request |
