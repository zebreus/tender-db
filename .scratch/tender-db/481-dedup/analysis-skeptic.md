## Rule definitions
Code is in `skeptic/lib.py`. Title equality means casefold plus collapsed whitespace.

- **A**: shared buyer organization_id AND equal title AND same notice subtype AND |Δ published| ≤ 7 d
- **B**: A AND the main-CPV set is exactly equal
- **C**: B AND the internal ref (BT-22 / DE1- or SDK01-ProcurementProject-ID) is not different (missing passes)
- **D**: B AND the internal ref is present on both sides and equal
- **E**: C AND earliest deadline not different AND lot count not different AND |Δ| ≤ 1 d
- **F**: shared buyer org AND ref equal (both present) AND CPV equal AND subtype equal AND |Δ| ≤ 7 d (no title)
- **G**: shared buyer org AND token-Jaccard ≥ 0.8 AND CPV equal AND subtype equal AND |Δ| ≤ 7 d
- **H**: E AND ref equal (both present) AND deadline equal (both present)

Values used:
- Positives: each notice's own values.
- Negatives: the DÖE notice's own values against the TED Tender's canonical values.
- Unmerged: canonical values on both sides.

**1:1 check**: the DÖE island fires on exactly one TED candidate, AND that candidate is fired by exactly one DÖE island.

## 1. Contract and award notices: 458 positives, 1,028 twin_known negatives
The negatives are capped at the 3 nearest competitors per DÖE notice.

"Twin removed" means: drop the true twin and count DÖE notices where exactly one sibling still fires. That is the false-merge rate if the twin were missing from TED.

| Rule | Positives fired /458 | Negative pairs fired | Unique correct / ambiguous / unique wrong / none | Twin removed: one sibling fires |
|---|---|---|---|---|
| A | 451 | 12 | 441 / 10 / 0 / 7 | 8 |
| B | 448 | 9 | 440 / 8 / 0 / 10 | 7 |
| C | 448 | 4 | 444 / 4 / 0 / 10 | 4 |
| D | 403 | 1 | 402 / 1 / 0 / 55 | 1 |
| E | 279 | 3 | 276 / 3 / 0 / 179 | 3 |
| F | 403 | 6 | 400 / 3 / 0 / 55 | 1 |
| G | 448 | 13 | 437 / 11 / 0 / 10 | 9 |
| H | 106 | 0 | 106 / 0 / 0 / 352 | 0 |

What these numbers can and can't show:
- **The doe_only_key_disjoint pool has no hard negatives.** It has 0 same-subtype pairs and 0 equal titles among 261.
- **A "TED Tender has no DÖE version" guard would hide the problem rather than fix it.** All 12 negative pairs that A fires have a TED side with its own DÖE twin (766 of the 1,028 negatives do). The guard removes all 12, but leaves the risk unmeasured: the 262 TED-only competitors hold 4 equal titles, all outside Δ ≤ 7 with the same subtype (Δ 13, 30, 7 with subtype 16 vs 4, 13).
- **Publication gap.** Only 280 of 458 positives fall within Δ ≤ 1 d, and 454 within 7 d.

## 2. PIN pairs with known answers (from BT-701 equality)
I matched each island's DÖE notice UUID against BT-701 on its TED candidates (3 reads).
- eForms-DE 1.x: 54 of 55 have a twin. eForms-DE 2.x: 73 of 73. EU SDK transport notices (T01): 9 of 9. National E2/E3: 0 of 50.
- Signal agreement on the 136 pairs:
  - Title, CPV, subtype, buyer org and dispatch second: all 136.
  - Internal ref: 117 agree, 19 missing, 0 differ.
  - Deadline: missing on 134. That is why H fires 0.
  - Publication gap: 94 within Δ ≤ 1, 135 within ≤ 7, maximum 11 d.

1:1 results (correct / wrong / not admitted):

| Rule | Correct | Wrong | Not admitted |
|---|---|---|---|
| A | 121 | 0 | 15 |
| B | 123 | 0 | 13 |
| C | 130 | 0 | 6 |
| D | 111 | 0 | 25 |
| E | 93 | 0 | 43 |
| F | 111 | 0 | 25 |
| G | 123 | 0 | 13 |

- With twin removed, every rule except E makes 1 wrong merge: Königssee T1588328 would go to its sibling T1198434 (5 days later, 1 lot instead of 3, same ref 51). E makes 0.
- Of the 184 pairs that A fires among unmerged eForms records, the internal ref agrees on 121, differs on 44 and is missing on 19.

## 3. False-merge shapes, with the signals each one defeats
1. **Lots published as separate procedures under one title.**
   - Bezirkskliniken Schwaben DLZ Günzburg: 7 DÖE PINs and 7 TED PINs, all with the same title, CPV 45215100, 1 lot, Δ 1 d. That gives 49 pairs under A, 42 of them wrong. Only the ref (82406-4100…4991), the lot title (Heizung, Sanitär, Lüftung…) and distinct documents URLs separate them.
   - Roggentin: 4 procedures with the same procedure title and description (one title has a typo), CPV 71000000, all on the same day. Defeats A, B and G. It also defeats C, because sibling T182702 has no ref. Only the deadline time (08:00 / 09:00 / 09:30), the lot title and the URL separate them.
   - Hanau (T276995 vs T453533): defeats A–G. Only BT-24 and the URL separate them.
2. **Framework lots modified on the same day** (Bw, 567147 ↔ 1142490, subtype 38): defeats A, B, C, E and G.
3. **Repeated modifications of one contract, each under a new BT-04** (Deichverband: 65832 vs 207474 at Δ 4 d, vs 192063 at Δ 13 d): the same title, lot title and description. Only the modification text (BT-201) and a ref prefix differ. Defeats A, B and G.
4. **A second procedure with the same title, beside a corrigendum** (DFN "Betriebsunterstützung NACS", 444151 'V 01_24_II' vs 803757): same title, lot title, description and CPV, Δ 5 d. 803757's corrigendum contract notice lands at Δ 0 from 444151's TED notice. Separated only by ref and deadline (04-24 vs 05-13).
5. **Re-published PINs** (ZVBN Ammerland Süd; Düsseldorf Heinzelmännchenweg, two DÖE and two TED notices each, identical in every field including the ref). Picking the nearest date is wrong for T1581303: it picks T1195763 at Δ −1, but the true twin is T1196204 at Δ +3.
6. **Internal ref is a project id, not a procedure id.** 17 of 1,028 negatives share the ref. Examples: Körse-Therme Kirschau (4 procedures), Asklepios NIDA 446-24 (3 award notices with the same lot title), Rosenheim 41-621-43, Pfarrkirchen 452, Ascheberg 1575/23. Defeats F, and any rule that uses the ref alone.
7. **Generic title with a date-coded ref** (Stuttgart "Gerüstarbeiten", 3210_EU_130525 vs _140525): defeats A, B and G. BA REZ NORD (subtype 33) differs only by lot count, 2 vs 6.
8. **Fuzzy titles add only false matches.** All 454 positive titles that agree are exact; fuzziness gains no recall. Allowing title similarity ≥ 0.95 raises the negatives (same subtype, Δ ≤ 7) from 12 to 28, and from 9 to 19 with CPV equal. Examples: Arnsberg "Saniierung" (0.991), Roggentin's typo (0.984), "Los 3" vs "Los 2" Raketenbauteile (0.962, same CPV, no ref), OBS-Mitte "tom-Brok Straße" vs "tom-Brok-Straße". Jaccard ≥ 0.8 lets in 22 negatives.
9. **Several notices dispatched in the same second.** 6 of 1,028 negatives share the dispatch second, for example EFRE VI Biobank (title similarity 0.968, same CPV) and Esslingen Gas vs Strom. So do 1 of 261 key-disjoint pairs and 1 of 1,327 PIN non-twins.
10. **Exact deadline isn't identity either.** It agrees for 206 of 210 positives, but also for 30 of 429 negatives (none of those also has an equal title) and 24 sdk-0.1 sibling pairs.
11. **A PIN against the later contract notice** (Löhne Boardinghouse: DÖE contract notice vs TED PIN, same title and CPV, Δ 7). Defeats any rule that drops the subtype requirement.
12. **sdk-0.1 national notice vs eForms-DE EU notice** (Lathen, T1582137 vs T122620, which already has its own DÖE twin): defeats name match, near title, same-day deadline and Δ ≤ 7.
13. **sdk-0.1 sibling procedures built from the same title template:**
    - Hessepark 5 vs Eilbektal 35 Trockenbau: same CPV and deadline to the minute.
    - ADBV Aichach vs FA Amberg: same CPV, Jaccard 0.64.
    - Adobe vs Broadcom licences.
    - Estricharbeiten Suitbertusstraße vs Heinsenstraße.
    - In the 630 sample: title similarity ≥ 0.9 within Δ ≤ 7 gives 1 pair, and adding CPV and deadline gives 0.
14. **sdk-0.1 numeric change notices are split into separate islands.** 110 base ids in April (230 islands) are split, and 0 are chained. Example: 23654900-1 / -2, deadline 04-25 → 05-05.
15. **UUID key collisions.**
    - TED 1110706 holds Stadt Wesel (2024) and Община Белоградчик (2025-04-29).
    - TED 42726 holds 36 versions across 8 buyer orgs.
    - 17 negatives share an org only through the TED Tender's other versions. A rule should compare the buyers named in each notice itself, not the Tender's accumulated party list.
16. **Joint procurement.** In all 7 positives with several buyers, both sides list the same buyer set.

## Implications
- Treat the eForms islands as a declared link: BT-701 equality plus dispatch time.
- A matched rule needs:
  - the 1:1 check;
  - exact titles;
  - a procedure-specific discriminator, such as the lot title, the documents-URL id, or the deadline to the minute.
- Calibrate per subtype family.

## Files
All in `.scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)skeptic/`:
- `lib.py` holds the rule definitions; the analysis scripts are `eval1.py` to `eval20.py`.
- `reqlog3.jsonl` logs the 18 requests, with raw response bodies in `r3/`.
- Data from the reads: `uuid_twins_ef.json`, `bt701_efcand.json`, `disp_neg_ef.json`, `urls_sdk.json`, `texts_shapes.json`, `texts_lathen_urls.json`, `collision.json`, `versions_lathen_dfn.json`.