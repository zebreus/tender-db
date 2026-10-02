## Scope and inputs
- I sent no requests to prod. Everything was computed with Python from positives.json (458), negatives.json (1,289: 1,028 `twin_known` and 261 `doe_only_key_disjoint`) and unmerged.json (817 records with 6,038 signalled candidates).
- Scripts and outputs are in `.scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)calib/`:
  - `feat.py` builds the features; `rules.py` holds the rule table.
  - `evalrules.py`, `bands.py` and `final.py` run the evaluation.
  - `results.json` has every band, with and without the guard, including the per-group unmerged counts. `rule_eval.json` covers all 41 rules. `feats.pkl` is the feature cache.

## Which values are compared
- **Positives:** each notice's own notice-layer values on both sides (canonical values are cumulative fold state on a merged Tender).
- **Negatives:** the DÖE side uses notice-layer values where present (`twin_known`) and canonical values otherwise (`doe_only_key_disjoint`, pure DÖE). The TED side uses canonical values.
- **Unmerged:** canonical values on both sides.
- Internal references come from notice_ids on every side.
- **Conservative choice:** every negative is treated as sharing the buyer org id and name, because the pool was built on `shared_org_ids`. 17 negatives' version-level buyer lists did not overlap, and they are counted as passing anyway.

## Exact signal definitions
- **`org`:** the two sides' `buyer_org_ids` sets intersect.
- **`name`:** the buyers' `name_norm` sets intersect.
- **`title norm`:** some title on one side equals some title on the other after NFKC normalisation, casefolding, replacing every run of characters outside `[0-9a-zà-ɏ]` with one space, and trimming. Exact case-insensitive equality gave identical counts.
- **`title jacc>=t`:** the highest token-set Jaccard similarity over all title pairs, using the normalised tokens.
- **`sub`:** the eForms subtypes are equal and not null. sdk-0.1 has a null subtype, so it never passes.
- **`dt`:** the absolute publication gap in days (the DÖE version against the TED version).
- **CPV main, deadline, value, internal ref, lots:** each signal is `eq`, `ne` or `na` (missing on either side).
  - CPV main: the sets overlap.
  - Deadline: the earliest deadline (`min`) is exactly equal.
  - Value: `procedure_cents` is exactly equal.
  - Internal ref: the sets overlap after NFKC normalisation, casefolding and removing all whitespace. The internal ref is BT-22-Procedure, DE1- or SDK01-ProcurementProject-ID.
  - Lots: the lot counts are equal.
- **`no-contra`:** none of CPV main, deadline, value, internal ref or lots is `ne`.
- **G1:** the target TED Tender has no DÖE version (`census_sources` lacks `doe`). It passes all positives, because the twin was TED-only before the key merged it.
- **Uniqueness:** a DÖE Tender is joined only when exactly one candidate fires; two or more is ambiguous.
- **Window:** every candidate is within ±30 days, the census window.

## Single signals with org-id blocking (TP out of 458, FP out of 1,289, precision lower bound)
| Signal | TP | FP | Precision lower bound |
|---|---|---|---|
| org alone | 457 | 1,289 | 0.242 |
| + same subtype | 457 | 909 | |
| + dt≤7 | 454 | 827 | |
| + CPV main equal | 450 | 176 | 0.682 |
| + NUTS overlap | 445 | 720 | |
| + deadline exact | 206 | 30 | 0.824 |
| + value exact | 67 | 8 | 0.803 |
| + internal ref equal | 407 | 17 | 0.937 |
| + title norm equal | 453 | 16 | 0.945 |
| + title Jaccard ≥0.9 / 0.8 / 0.7 / 0.6 / 0.5 | 453 / 454 / 454 / 454 / 454 | 22 / 35 / 73 / 122 / 157 | |
| + title SequenceMatcher ratio ≥0.95 / 0.9 | 453 / 454 | 36 / 63 | |

The 30 deadline-exact false positives are same-buyer sibling trades that share a submission deadline.

## Bands, strictest to loosest
Two terms used in this section:
- **Gated:** pairs that pass org, same subtype and dt≤7. There are 658 gated negatives.
- **Twin-absent:** each positive's DÖE notice scored against only its up to 3 nearest same-buyer TED competitors (380 queries), as if its TED twin were missing.

- **L1:** `org & title norm & sub & dt≤7 & no-contra & iref eq & deadline eq`.
  - TP 184 (40.2%), FP 0, precision lower bound 0.9795.
  - Unmerged: 1 join (subtype 7).
- **L2:** `org & title norm & sub & dt≤7 & no-contra & deadline eq`. Dropping the title (C15: `org & deadline exact & sub & dt≤7 & no-contra`) gives identical numbers.
  - TP 203 (44.3%, Wilson 95% lower bound on recall 39.8%), FP 0 of 1,289 (upper bound on false-positive rate 0.30%), 0 of 658 gated (upper bound 0.58%).
  - Precision 1.0, lower bound 0.9814. Twin-absent wrong joins: 0 of 380 (upper bound 1.0%).
  - Unmerged: 2 joins, 0 ambiguous.
  - **This is the highest-recall band with 100% sample precision without G1.** If value is dropped from `no-contra`, TP rises to 204 with FP still 0.
- **Lv:** `org & cpv eq & deadline exact & value exact`.
  - TP 32, FP 0, lower bound 0.893. Unmerged: 0 joins.
- **L3:** `org & title norm & sub & dt≤7 & no-contra & iref eq`.
  - TP 400 (87.3%), FP 1, lower bound 0.9860. With G1: FP 0, lower bound 0.9905.
  - Unmerged: 112 joins (107 eForms-DE, 5 T01), 4 ambiguous.
- **L4:** `org & title norm & sub & dt≤7 & no-contra & (iref eq OR deadline eq OR value eq)`.
  - TP 422 (92.1%, recall lower bound 89.3%; 93.5% on the 367 cleanly paired positives — those with exactly one DÖE and one TED version of the paired subtype).
  - Rule alone: FP 1 of 1,289 (upper bound on false-positive rate 0.44%), 1 of 658 gated (upper bound 0.86%). Precision 0.9976, lower bound 0.9867.
  - Per query with uniqueness: 421 correct joins, 1 refused as ambiguous, 0 wrong. Twin-absent: 1 wrong join of 380 (upper bound 1.48%).
  - With G1: FP 0, lower bound 0.9910, twin-absent 0 wrong.
  - Unmerged: 115 joins (110 eForms-DE, 5 T01), 4 ambiguous.
- **L5:** `org & title norm & sub & dt≤7 & no-contra`.
  - TP 445 (97.2%), FP 3, lower bound 0.9805. With G1: FP 0, lower bound 0.9914.
  - Unmerged: 131 joins (122 eForms-DE, 9 T01), 4 ambiguous.
- **L6:** `org & title norm & same subtype & dt≤7`.
  - TP 451 (98.5%), FP 12, lower bound 0.9552. With G1: FP 0, lower bound 0.9916.
  - Unmerged: 121 joins, 14 ambiguous.
  - **This is the highest-recall band with 100% sample precision under G1**, but only 63 of G1's surviving negatives pass the other gates (upper bound 5.75%).
- **L7:** `org & title norm`.
  - TP 453, FP 16 (4 with G1), lower bound 0.9453.
  - Unmerged: 109 joins, 27 ambiguous.
- **L8:** `org & title Jaccard ≥0.8`.
  - TP 454, FP 35 (9 with G1).
  - Unmerged: 107 joins, 29 ambiguous.
- **L9:** `org & CPV main equal`.
  - TP 450, FP 176 (50 with G1), lower bound 0.682.
  - Unmerged: 90 joins, 61 ambiguous.

How much G1 is worth: it removes 944 of the 1,289 negatives outright, because their TED Tender already has a DÖE version. Only 345 negatives survive it, 63 of them gated, and 39 positive queries have a TED-only competitor that passes the gates. So the guarded 0-FP results above have upper bounds of 1.10% (out of 345) and 5.75% (out of 63).

Projection for L4 on the eForms family: 292 gated candidate pairs, of which 115 become joins. A 0.86% upper bound on the false-positive rate leaves at most 1.5 expected false fires (95%), which puts precision on the 115 joins at 98.7% or better. This holds only if the rate measured on contract and award notices carries over to prior-information notices.

## Applying the bands to unmerged.json
The eForms family (128 eForms-DE and 9 T01) and the 330 sdk-0.1 numeric EU-legal-basis records are a full census of April 2025. The other sdk-0.1 records (200 numeric national, 100 uuid) are random samples.

- **L4's 115 joins:**
  - Subtypes: subtype 4 60, subtype 7 35, subtype 5 11, T01 5, subtype 1 2, subtype 6 1, subtype 8 1. None are subtype 16 or 29.
  - Every target is a TED island with no DÖE version.
  - Publication gap: 0 days 14, 1 day 67, 2 days 17, 3 days 11, 4 days 4, 5 days 2.
  - What agreed besides the title: internal ref only 98, internal ref and value 13, value only 2, internal ref and deadline 1, deadline only 1.
- **Ambiguous (4):** two 2×2 groups of prior-information notices (ZVBN "LB AM Süd", and BEK-2025-0004). In each, two DÖE prior-information notices and two TED islands share the same title and internal ref.
- **Hidden candidates:** 0 joins could hide further candidates. None of the 3,974 candidates read without signals shares the DÖE record's subtype, so bands L1–L6, which all require the same subtype, see every candidate that could fire.
- 0 TED Tenders are claimed by two DÖE Tenders in L1–L8.
- **sdk-0.1** (530 numeric, 100 uuid): 0 joins under every band from L1 to L8.
  - Org-id candidates: 0 numeric, 19 uuid.
  - `name & title norm`: 0, because no candidate has an identical title.
  - The loose `name & deadline exact & CPV equal & no-contra` scores TP 203 / FP 0 on the calibration sample. It joins 1 numeric record, which is false ("Hessepark 5 - Trockenbau" against "Eilbektal 35 - Trockenbau", 10 days apart), and leaves 1 ambiguous.
- **EU SDK 1.0** (50): 0 joins under L1–L8.

## Limits of the calibration
1. **No prior-information notices among the positives.** The positives are subtypes 16 (245), 29 (139), 17, 38, 30, 39 and similar; prior-information subtypes appear only as subtype 10 (3 pairs), and subtypes 1–9 not at all. The negatives' DÖE sides are all subtype 10 or above. The joins are 100% prior-information notices and T01, so they need hand reads of the boundary cases, or the exact key in point 4.
2. **Noisy positive labels.** 91 of the 458 positive Tenders contain more than one DÖE or TED version of the paired subtype: one BT-04 shared by several trades, for example Sanitär against Heizung in Tender 1052491. The 4 positives whose titles differ are nearest-in-time sibling mispairings, not twins with different titles. Recall on the 367 cleanly paired positives is in the band list above.
3. **Hard negatives the rules must survive.**
   - Kita Am Hochgericht (276995 against 453533): same buyer, title, internal ref, CPV and subtype, 1 day apart.
   - Krankenhaus-Bekleidung (567147 against 1142490, subtype 38): same title and CPV, no internal ref.
   - Roggentin (614979 against 988067 and 182702): identical title, internal refs 2025-24-BA against 2025-26-BA, deadlines 1–1.5 h apart.
4. **A possible exact key, measured on only 2 pairs.** The TED notice's `BT-701-notice` equals the DÖE publication_id minus its version suffix (`-NN`):
   - Tender 1005906: TED notice 24707855 carries 1023d593-…, the DÖE publication id of notice 26631787.
   - Tender 697984: TED notice 24724249 carries 374e7ae0-…, the DÖE publication id of notice 26617363.

   It is not measured on the 458 positives or on the unmerged candidates. Checking it needs `notice_ids` reads with `+field_id='BT-701-notice'`, arms by notice_id.
5. **Precision on the sample depends on the 458:1,289 mix.** The per-pair false-positive rate and its upper bound are the figures that carry over.