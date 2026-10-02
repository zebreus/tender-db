## Scope and inputs
- **Prod requests:** 0. The computed task stated no request count, so none were sent. No repo edits, no cargo.
- **Inputs:** positives.json (458), negatives.json (1,289), unmerged.json (817), ted_notices_chk.json (TED dispatched_at for the 458 positive TED notices plus 300 candidate notices) and notices_doe.json.
- **Island labels:** these come from another lens's existing read, ../skeptic/bt701_efcand.json (BT-701/BT-04 for 795 TED candidate notices). I recomputed the UUID match offline (DOE publication_id without its -NN suffix = TED BT-701-notice); it gives 0 disagreements with ../skeptic/uuid_twins_ef.json.
- **My scripts and outputs** are in .scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)recall/: rf.py (features), bands.py, miss.py, island.py, fixes.py, r1.py, fixes.json, F.pkl, twins_ef.pkl.

## Atom definitions (exact)
- **O:** the two sides' buyer organization_id sets intersect. Buyers are tender_version_parties rows with role LIKE '%uyer%'. For positives: provenance-exact rows of each notice. For negatives and candidates: the TED canonical rows at the named seq.
- **T:** some title pair is equal after norm(s) = NFKC, then casefold, then every run of characters outside [0-9a-zà-ɏ] replaced by one space, then trim.
  - Positives use the notice-layer procedure title (BT-21-Procedure / DE1- or SDK01-ProcurementProject-Name), any language key.
  - Everything else uses the canonical title text (lot_id NULL), any language.
- **S:** notice subtype equal and non-null.
- **D7:** |TED.published_at − DOE.published_at| ≤ 7 days.
- **D7e:** min(|TED.pub − DOE.pub|, |TED.pub − DOE.dispatched_at|) ≤ 7 days.
- **NC:** no field that is present on both sides and differs, among:
  - CPV main sets disjoint;
  - deadline minimum unequal (exact second);
  - procedure estimated value unequal (exact cents);
  - internal_ref sets disjoint (NFKC, casefold, whitespace removed);
  - lot count unequal.
- **NC−val:** NC without the value check.
- **Corroborators:**
  - DL: deadline minimum equal.
  - IR: internal_ref sets intersect.
  - VL: procedure value equal.
  - DPs: notices.dispatched_at equal to the second, and the DOE dispatch is not on a full hour (t mod 3600 ≠ 0).
  - DPh: dispatched_at equal and on a full hour.
- **G1:** the candidate TED Tender has no DOE version.

## Strict bands
These are the same bands as calib's L1–L5 (C13/C11/C10/C12/C8); I re-derived identical TP/FP.

| Band | Rule | Positives TP of 458 | FP of 1,289 | FP with G1 |
|---|---|---|---|---|
| L1 | O&T&S&D7&NC&IR&DL | 184 | 0 | 0 |
| L2 | O&T&S&D7&NC&DL | 203 | 0 | 0 |
| L3 | O&T&S&D7&NC&IR | 400 | 1 | 0 |
| L4 | O&T&S&D7&NC&(IR\|DL\|VL) | 422 | 1 (276995→453533) | 0 |
| L5 | O&T&S&D7&NC | 445 | 3 (276995→453533; 567147↔1142490) | 0 |

## Why the strict bands miss positives
- **L2, 255 misses:**
  - deadline absent on both sides: 248 (subtype 29: 139, 16: 50, 38: 17, …);
  - no corroborator at all: 28;
  - CPV contradiction: 7; title differs: 4; deadline contradiction: 4; Δpub > 7 days: 4; value contradiction: 3; internal-ref contradiction: 3; org split: 1; subtype differs: 1.
- **L4, 36 misses:** 23 have no corroborator (no deadline and no internal ref); the other 13 are the L5 misses below.
- **L5, 13 misses:**
  - **8 pairing artifacts.** Dispatch differs, so the paired TED version is a different notice. These are multi-notice Tenders that reuse one BT-04 across separate CNs with different titles, CPV and internal refs: 718063 DLR, 277690 DLR, 1052491 Markt Goldbach, 1005511 VG Alzey-Land, 779070 Schneverdingen, 389466 Stadt Roth, 443341, 889083. All 7 CPV contradictions, all 4 title differences, all 4 deadline differences and all 3 internal-ref differences in the positives sit in these 8.
  - **3 value scope:** 957571, 117566, 315838. The DE1 DOE "procedure" value is a lot value; for example 315838 has DOE 250k against TED procedure 1.2M with a TED lot sum of 250k.
  - **2 DOE published_at anomaly:** 5056 (Δpub 15 days) and 1164525 (21 days). In both, the dispatch is identical and the DOE published_at is weeks before its own dispatch.

## True twins (dispatched_at equal, 449/458)
These are the pairs that are the same eForms notice. Of the 449:
- T, CPV main, S, O, lot count and legal basis each agree in 449/449;
- NUTS equal 438, missing 11;
- internal ref equal 404, missing 45, differing 0;
- deadline equal 204, missing 245, differing 0;
- value equal 67, missing 376, differing 3;
- Δpub > 7 days: 2.

So the true pairs that "look different" are 5/449 (1.1%): 3 value scope and 2 DOE pub-date anomaly. Translated titles 0, corrigendum deadline edits 0, CPV differences 0, org-id splits 0, subtype differences 0.

## Gain per fix
| Fix | Positives | Islands | FP |
|---|---|---|---|
| Fd: DP as corroborator | L2 203→437 (+234); L4 422→445 (+23) | +11 (TED dispatch known for only 12 of 136 twins) | +0 |
| Fv2: value removed from NC | L2 +1, L4 +3, L5 +3 | +0 | +0 (true twins: value differs in 3 of the 70 pairs that carry a value on both sides, 4.3%; negatives: 94 of 1,289 differ) |
| Fv: scoped value (any equality among procedure and lot-sum values) | L2 −1 | — | — |
| Ft: D7e instead of D7 | L4/L5 +2, L2 +0 | +1 | 3 of 462 negatives with Δpub > 7 days flip; none passes the other atoms |
| Fo: O ∨ name_norm shared | +0 | +0 (all 136 twins are found via org id) | Unmeasurable: the negatives are org-id-blocked |
| All TED versions of the candidate Tender | at most +7 (of the 9 pairings where dispatch differs, 7 have 3–16 other same-subtype TED versions in the window; 389466 and 889083 have their twin outside ±30 days) | +0 (twins are single-version islands) | Unmeasured: needs every version of each negative TED Tender |

- **Fv** loses 749469 (DOE procedure 840k against TED lot sum 2.6M across 7 lots), so it is worse than dropping the value check.
- **Fo:** the only org split in the positives is 889083. It is a CAN against a CN, not a notice twin, joined in prod via OPP-090 (DOE OPP-090 = 313536-2025). The split comes from phone-number identifiers, 09771616015 against 499771616017.
- **All versions:** across unmerged candidates, 1,356 of 10,012 have 2 or more TED versions in the window.

Combined:
- L2+Fd+Fv2+Ft: 442/458, FP 0. It misses 7 hour-grain dispatches and the 9 pairings where dispatch differs.
- L4+Fd+Fv2+Ft: 450/458, FP 1 (0 with G1). All 8 misses are pairing artifacts.

## Recommended band R1
R1 = O & T & S & D7e & NC−val & (DL | DPs | (DPh & IR))

- **Positives:** 449/458 (98.0%, Wilson 95% [96.3, 99.0]%).
- **True twins:** 449/449 (LB 99.15%).
- **What admits each pair:** IR+DPs 212, DL+IR+DPs 175, DPs only 26, DL+DPs 19, DL+IR+DPh 10, IR+DPh 7.
- **The 26 DPs-only admits are the boundary for a review read:** 34355, 63241, 76983, 85427, 127787, 175213, 176393, 292579, 319691, 327601, 367280, 385455, 393975, 427198, 428646, 463198, 474902, 567147, 608718, 886852, 1012065, 1045140, 1104520, 1113899, 1142490, 1153069.
- **FP:** 0/1,289 (Wilson UB 0.30%). Three negatives pass the core.
  - 567147↔1142490 are two framework agreements with identical titles; their dispatch differs (10:12:34 against 10:21:07), and dispatch picks the right twin.
  - 276995→453533: the TED notice 24713010's dispatch is not in the data. It is inferred different, because its own DOE twin 26628262 was dispatched at 06:23:41 against 06:41:06. One PK read would confirm it (SELECT id,dispatched_at FROM notices WHERE id=24713010).
- **Dispatch collisions:** equality at sub-hour grain among known non-twin same-buyer pairs is 0 of 155 negatives plus 0 of 266 island non-twin candidates, 0/421 in total (UB 0.90%). Hour grain collided in 1 of the 2 known negatives (2025-04-29T22:00:00Z), which is why DPh needs IR.

## Unmerged DOE-only Tenders
**eForms family (a census for April 2025):**
- 136 of 137 eForms-DE (EU legal basis) and T01 records have exactly one TED twin with BT-701 = DOE notice UUID: de-1.1 29, de-1.2 25, de-2.0 73, sdk-1.12 8, sdk-1.13 1. The other 50 sdk-1.0 records and the 1 de-1.2 national record have none.
- All 136 twins are TED islands with no DOE version, so G1 costs nothing here. Every twin passes the core (O&T&S&D7e&NC−val).

| Band | Correct join | Ambiguous | Wrong |
|---|---|---|---|
| L2 | 2 | 0 | 0 |
| L4 | 115 | 4 | 0 |
| L5 | 131 | 4 | 0 |
| L5+Ft | 132 | 4 | 0 |

- **L2 refuses 134:** none of them has a deadline, because they are PINs (subtype 4: 70, 7: 41, 5: 11, T01: 9, …). 18 also have no internal ref.
- **L4 refuses 17:** 16 have no corroborator; 1 is the Δpub anomaly (1584140, published 04-12, dispatched 04-19).
- **The 4 ambiguous** are 2 PINs re-published twice on both portals (Ammerland Süd 1576759/1577989; Heinzelmännchenweg 1579943/1581303). Dispatch equality resolves the Ammerland pair; the TED dispatch of the other pair is not in the data.
- The 51 non-twin records get 0 joins under any band.
- The declared BT-701 link recovers all 136. Against the 12,513 merged that month, these 136 are a 1.08% gap in the key rule.

**sdk-0.1 (630 sampled out of 13,465 DOE-only):**
- Subtype is NULL on 630/630, so S fails every time.
- No numeric record shares an org id with TED; only 19 of the 100 uuid records have an org-id candidate.
- Name-based candidates exist for 342 records. Title equal: 0. Jaccard ≥ 0.8: 1 (1582137 Lathen against 122620, Jaccard 0.89, deadlines 10 minutes apart, national legal basis).
- **EU-basis 330:** 125 have no same-name TED Tender within ±30 days, and the rest have a maximum Jaccard of 0.67.
- **Deadline equality alone is not a safe signal:** 8 records share an exact deadline with a different-procedure TED Tender of the same buyer name.
- **Gain from every fix above on sdk-0.1:** 0 of 630; at most 1 with a fuzzy title. The blocking stage, not the rule, is the gap.
- **Unmeasured lead:** the documents URL (SDK01-…CallForTendersDocumentReference…URI against TED BT-15-Lot). It is present in the discovery picks but was never read for pairs.

## Gaps and anomalies
- **TED dispatch is mostly unknown** for negatives (known for 157/1,289) and island twins (12/136). The number of PK reads that would close this (about 1,256 notice ids, roughly 4 requests in the UNION ALL shape stage10 used) is an estimate.
- **DOE published_at precedes dispatched_at by 2 days or more:**
  - positives 5/458 (de-2.0 3, de-2.1 2);
  - DOE notices 26740443 and 26772363 were dispatched 2025-08-22 and 2025-10-01 but carry an April published_at;
  - unmerged eForms-DE 1/128; sdk-0.1 12/288.
- **443341:** the DOE notice's BT-04 (3484f8c6…) differs from the Tender key (60cab6eb…). How it joined is not visible in the sample.