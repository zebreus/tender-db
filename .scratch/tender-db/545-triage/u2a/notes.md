# 545 unit 2a: GB-PPON candidate listing (2026-10-19)

This unit was read-only. It used 9 of its 12 `/v1/sql` requests, with no 408 and no retry (`requests.log`), plus 1 public GET (`api_requests.log`). Nothing was POSTed, and no code, git, box write or admin job was touched. Before any read went to the box, both planners checked its plan locally on a copy of v551's plan.db, after the two prod org indexes were added to that copy: python sqlite3 (`planprobe.py`) and turso (`tplan.sh`, which wraps `target/debug/plan-probe plan`). The plans are in `*.plan.txt`; every step is a SEARCH and none is a SCAN. `run.sh` refuses to send a read with no local plan, a read whose plan contains a SCAN, and any read beyond the 12th.

## Coverage (exact)
- Slices s1–s6 covered every organization with an identifier in ['GBPPON', 'GBPPOO'), except ['GBPPONPC', 'GBPPONPE'), which was unit 1's q3 slice:
  - s1: GBPPON0..PB and PG..PH
  - s2: PJ..PM
  - s3: PN..PQ
  - s4: PR..PV
  - s5: PW..PX
  - s6: PY..PZ and RD
- That is 12,761 orgs, equal per prefix to q2's population count (all 21 prefixes match). Of those, 7,865 have 2 or more mentions. With unit 1's 1,577 orgs, all 14,338 GBPPON-keyed orgs are now covered.
- Not covered: GB-PPON mentions on orgs keyed some other way. The 456 exhibit itself is one of these: Sellafield is org 5718658, keyed by the bare number `01002607`, so neither 2a nor 2b would ever list it. It was read on purpose in r7/r8.

## Pipeline (all scripted)
1. `candidates.py`
   - **STRAY:** the census rule: a key held exactly once, under a top key held 2 or more times, that agrees with no other key.
   - **HELD-2:** the twice-held foreign-key check, now written as code: a key held 2..K times (K=2, env `HELD_K`), held fewer times than the top key, and agreeing with no other key.
   - **Prefilter:** applied to both detectors. The candidate's distinctive tokens must be disjoint from those of the head and of every more-established key. An empty token set is kept.
   - **Validation on unit 1's slice (`val_q3`):** reproduces its 73 strays. All 4 by-eye twice-held keys are found (CEBR, Pagabo, Wavemaker, Charlton Park). Supplier-side F recall is 8/8. Only the Tees trio is dropped, and the trio is U.
   - **On the COH slice (`val_q8`, for 2b):** 1 HELD-2 is kept: "LITE Limited" ×2 on Lighting & Illumination Technology Experience.
2. Output for s1–s6:
   - 650 strays: 612 GB-PPON and 38 GB-NHS.
   - 285 HELD-2 hits.
   - The prefilter kept 208 (140 STRAY, 68 HELD-2) and dropped 727. The dropped hits were not reviewed; their recall is inferred only from unit 1's labels.
3. `judge2a.py` hand-classifies the 208 kept candidates:

   | Class | Keys | Mentions |
   | --- | --- | --- |
   | V | 58 | 69 |
   | A (agent and principal) | 52 | 79 |
   | T | 27 | 33 |
   | U | 22 | 25 |
   | F? | 9 | 11 |
   | **F** | **40** | **47** |

   Northern Housing Consortium alone accounts for 34 A keys: member councils and housing associations named on NHC's PPON.
4. `verdicts.py` applies the guards as code:
   - **G1, shared-section collision.** The roles of every party and bid-party row bound to the candidate's (notice, section) come from r7, or from q6 and r9 for carried candidates. More than one role parks the candidate. This triggered 0 times in this half: every F section carries a single role. Two holders also appear on the same notice in their own section (Frimley GB-NHS-RDU as buyer beside INTUITIVE on the PPON section; SEHSCT's hyphenated PPON section beside PPL/PRS's unhyphenated one), so the section ids differ and nothing collides.
   - **G2, buyer-side F on a public buyer's identifier.** Council on council, trust on trust, a body on a department. These park as possible shared procurement: 7 keys, 10 mentions.
   - **G3, destination.**
     - (a) Exactly one identified row with the same country and the same vote key.
     - (b) Several such rows: the most-referenced (mention count, r9), provided it is the unique maximum and the rows do not carry different company numbers (`distinct_entities`).
     - (c) No identified row: provisional rows with exactly the mention's name_norm and the same country. One row, or the most-referenced unique maximum, is accepted. A provisional destination is refused when an identified near-match exists (`near_identified`).
     - Otherwise the candidate parks.
   - **Class rule.** U, A and F? always park. V, T and C are NOT-545.

## Result (`verdicts.json`, readable `verdicts.txt` with per-candidate evidence: notice, section, roles, tender, destination pool with refcounts, near-match hints)

**REHOME-HIGH: 28 keys, 31 mentions.** 22 keys are new in 2a and 6 are carried from unit 1. By destination rule: 10 identified-unique, 11 identified tiebreak, 7 provisional-exact-unique. 3 are buyer-side, all buyer names sitting on a supplier's PPON: CUH ×2 on Teleflex, and Swansea Bay UHB on Storz.
- Carried from unit 1:
  - Jeol on UKRI → 12991212 "00939456", the most-referenced of 4 rows (8 mentions; 31620129 has 2).
  - Portsmouth → 30971975.
  - EEP → 30928130.
  - PPI → 31587812 (provisional).
  - Watkins → 2479435 (provisional; the only GB row of the two).
  - Wavemaker ×2 on MoJ → 31555041 GBCOH04078547.
- New in 2a: Abbott Medical UK, Thomas Silvey, TIAA, NLA Media Access, PPL PRS, Fire Service College, Red Kite, CUH ×2, Hologic ×2, Renishaw ×2, Xiel, Tech Against Terrorism, ERMC, Vohkus, Bolton NHS FT, Gama, Virgin Media, Compendium, RSS International, The Law Society, Swansea Bay UHB.

**Parked or not the 545 class:**
- PARK-no-destination: 11 keys. These include Abri, INTUITIVE LIMITED, RCD Lockindge (typo; near hint 31534569), CollectionHQ ×2, M Group, QCI (carried), and Charlton Park Academy ×2 (carried: no row at all).
- PARK-destination-ambiguous: 2 (LSE ×2: identified "London School of Economics" 30974649; Taylor Wimpey).
- PARK-shared-procurement: 7.
- PARK-agent-principal: 53.
- PARK-sub-unit/group/sister: 25. This includes the Tees trio.
- PARK-relation-unknown: 11. This includes Derek Lewis and Healthcare 21 NI.
- NOT-545: 86.

**Issue 456's exhibit is PARKED as distinct entities.** On tender 8751605 (notice 46966800, section ORG-GB-PPON-PWYP-8439-MZWY), the mention "Schneider Electric" is the single Tenderer and winner, and it is bound to 5718658 Sellafield Ltd. That was read live today, so 456 is still open. Two GB identified rows match: 12539471 "Schneider Electric Ltd" 02886434 (10 mentions) and 30916476 "SCHNEIDER ELECTRIC LIMITED" GBCOH01407228 (33 mentions). These are different company numbers, so the tiebreak must not choose between them. Unit 3 should pick the entity from the register, or leave the mention parked.

## Notes for unit 3 / 2b
- The tiebreak's guard against company numbers that differ is new: the skeptic asked only for most-referenced plus same country. It is the reason 456 parks.
- The A class is large and systematic, and it breaks the "one-off copy-paste" premise:
  - NHC with its members;
  - NWSSP on health boards;
  - Star, Ardal and Welland;
  - brokers such as Ginger Energy, RPJ3 and Invictus.

  It needs its own decision, and should not be rehomed under 545.
- Org 30926847 (head "Network Rail") holds a PPON that 7 or more firms publish as Tenderer, so it is a magnet PPON. "Bupa Insurance Limted" ×2 ties the top key, which means neither detector flags it.
