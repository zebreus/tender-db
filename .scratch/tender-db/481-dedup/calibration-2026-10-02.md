# Issue 481, unit 1: TED↔DÖE cross-source match calibration (April 2025)

Requests for this synthesis: 0 `/v1/sql`, no repo edits, no cargo. I also re-ran three of the earlier scripts offline against files already saved, with no network calls: `skeptic/eval15.py`, `skeptic/eval16.py` and `recall/r1.py`. Wherever a figure comes from those re-runs, it is marked "re-run".

## Headline
- **Measured miss in April 2025:** 136 above-threshold DÖE-only Tenders have a TED twin and are not merged. The month had 12,513 merged Tenders, so this is 1.08 %.
  - All 136 are eForms prior-information-notice (PIN) or T01 islands.
  - All 136 are joined by a published identifier, not by fuzzy matching: TED `BT-701-notice` equals the DÖE notice UUID, which is the DÖE `publication_id` minus its `-NN` version suffix.
  - Extrapolated: about 2,650 across the population, interval [2,220; 3,130]. That would move "merged with TED" from 26.4 % to about 26.7 %.
- **Best matched rule (R1):**
  - 0 false positives among 1,289 negatives; the upper bound on the per-pair rate is 0.30 %.
  - Recall 449/458 = 98.0 % [96.3; 99.0]. Precision lower bound (Wilson) 99.15 %.
  - It needs no G1 guard.
  - In April 2025 it joins nothing that BT-701 does not already join, so its marginal yield is 0 Tenders.
- **sdk-0.1:** 0 joins under every rule tested. The gap there is the candidate search (blocking), not the matching rule, and it is not measured.
- **On "really really sure":** fuzzy titles gained 0 recall and added only false matches. Exact signals already reach 98 % recall. So the merge rule stays exact, and fuzzy matches go only into the `possible_duplicate_of` signal.

## 1. The sample

**Period and census**
- April 2025 DÖE versions. TED candidates within ±30 days; the census window holds 182,530 Tenders.
- 27,660 DÖE versions fall in April, giving 26,169 Tenders with an April DÖE version.
  - **Merged: 12,513.** All are eForms-DE, which is 98.9 % of eForms-DE.
  - **No TED version in the window: 13,656.** Of these, 12,763 are sdk-0.1 numeric, 702 sdk-0.1 uuid, 132 eForms-DE and 59 EU SDK.

**Positives: 458**
- Stratified random sample of merged eForms-DE Tenders (seed 4810): 1.1 = 120, 1.2 = 120, 2.0 = 170, 2.1 = 48.
- Each side is compared on that notice's own notice-layer values.
- The TED side is the same-subtype version nearest in time; 454 pairs are within 7 days.

**Negatives: 1,289.** Each pair is two different Tenders with the same buyer org id, published within ±30 days.
- 1,028 `twin_known`: the DÖE notice of a merged Tender against up to 3 other TED Tenders of the same buyer.
- 261 `doe_only_key_disjoint`.

**Unmerged: 817 records, with 10,012 TED candidates (6,038 carrying signals)**
- Full April census: 128 eForms-DE, 59 EU SDK (50 SDK 1.0 national and 9 T01), and 330 sdk-0.1 numeric with an EU legal basis.
- Random samples: 200 sdk-0.1 numeric national and 100 sdk-0.1 uuid.
- Four sampled Tenders that merged after the window were dropped.

**Reads**
- 251 sampler requests, 18 skeptic requests, and 25 from the earlier run. 0 error bodies, 0 HTTP 408.
- The calibration and recall-gap analyses sent 0 requests.

**Limits**
- **One month.** The interval below covers counting noise only, not month-to-month variation or the 2023-10 → 2025 eForms ramp.
- **No PIN positives.** The positives are contract and award notices (subtypes 16, 29, 17, 38, …). Subtypes 1–9 are absent, and subtype 10 has only 3 pairs. Yet every join in the unmerged set is a PIN or T01. The 136 BT-701 pairs stand in as PIN ground truth.
- **91 positives are noisy:** several versions of the paired subtype under one BT-04. 367 positives are cleanly paired.
- **The negatives' DÖE side is not DÖE-only.** The 261 key-disjoint pairs contain no hard negative: 0 share a subtype and 0 have equal titles.
- **No sdk-0.1 positives.**
- **Unmerged records and candidates use canonical values,** which are cumulative fold state.
- **Index use is inferred from timings.** The maximum wall time was 3.83 s.
- **Dispatch data differs between the two analyses.**
  - The recall-gap analysis had TED `dispatched_at` for only 157 of 1,289 negatives and 12 of 136 twins.
  - The skeptic's read (`skeptic/disp_neg_ef.json`, 1,761 TED notices) covers most negatives and all 136 twins.

## 2. Measured signals

**Single signals, every pair blocked on a shared buyer org id**

TP is out of 458 positives, FP out of 1,289 negatives.

| Signal added to `org` | TP | FP | Precision LB |
|---|---|---|---|
| (org alone) | 457 | 1,289 | 0.242 |
| CPV main equal | 450 | 176 | 0.682 |
| deadline exact | 206 | 30 | 0.824 |
| value exact | 67 | 8 | 0.803 |
| internal ref equal | 407 | 17 | 0.937 |
| title normalised equal | 453 | 16 | 0.945 |
| title Jaccard ≥ 0.8 | 454 | 35 | — |

**Bands**

Shared notation for this and later sections:
- `core` = org & title normalised equal & same subtype & Δpub ≤ 7 d & no contradiction (`NC`).
- `NC` means none of CPV main, deadline, value, internal ref or lot count is present on both sides and different.
- `IR` = internal ref equal; `DL` = deadline equal; `VL` = value equal; `DP` = dispatch time equal to the second.
- `D7e` = Δ ≤ 7 days, measured to the DÖE publication or the DÖE dispatch, whichever is closer.
- **G1:** the target TED Tender has no DÖE version yet.
- Islands = the 136 eForms islands that have a known BT-701 twin; the columns read correct / ambiguous / wrong.

| Band | Rule | TP /458 | FP /1,289 | FP with G1 | Precision LB | Islands |
|---|---|---|---|---|---|---|
| L1 | core & IR & DL | 184 | 0 | 0 | 0.9795 | 1 / 0 / 0 |
| L2 | core & DL | 203 | 0 | 0 | 0.9814 | 2 / 0 / 0 |
| L3 | core & IR | 400 | 1 | 0 | 0.986 | 112 joins, 4 ambiguous |
| L4 | core & (IR \| DL \| VL) | 422 | 1 (Hanau) | 0 | 0.9867 | 115 / 4 / 0 |
| L5 | core | 445 | 3 | 0 | 0.9805 | 131 / 4 / 0 |
| L6 | org & title & subtype & Δ ≤ 7 d | 451 | 12 | 0 | 0.9552 | 121 joins, 14 ambiguous |
| L7 | org & title | 453 | 16 | 4 | 0.9453 | 109 joins, 27 ambiguous |
| **R1** | core using D7e and no value check, & (DL \| DPs \| (DPh & IR)) | **449** | **0** | 0 | **0.9915** | 13 / 0 / 0, limited by dispatch data |

G1's 0-FP results rest on thin evidence. G1 removes 944 of the 1,289 negatives, so the true upper bounds are 1.10 % (345 negatives left) and 5.75 % (63 gated negatives left). The skeptic also shows that G1 hides the risk rather than measuring it.

**Notice-pair check on the 136 BT-701 PIN pairs** (skeptic rules A–G; columns are correct / wrong / not admitted)
- A: 121 / 0 / 15. C: 130 / 0 / 6. E: 93 / 0 / 43.
- With the twin removed, every rule except E makes 1 wrong merge: Königssee T1588328 goes to its sibling T1198434, which has 1 lot instead of 3.

**Dispatch to the second**
- True twins agree: 449/449 dispatch-identical positives, and 136/136 BT-701 pairs.
- Different notices also collide sometimes: 6 of 1,028 `twin_known` negatives, 1 of 261 key-disjoint pairs, and 1 of 1,327 island non-twins (re-run of `eval15.py`).
- None of those colliding pairs passes R1's core; only 3 negatives pass the core at all. So dispatch is a corroborator, never a link on its own.

## 3. Recommended admitted set

### 3a. Declared link (not a fuzzy band): notice-UUID equality
**Rule.** TED `BT-701-notice` equals the DÖE `publication_id` with `-<VersionID>` removed. The link must be one-to-one in both directions. Dispatch to the second is the consistency check.

This is an exact published identifier. ADR-0003's 2026-07-19 verification already names "identical notice and procedure UUIDs".

**Evidence**
- 136 of 136 EU-basis eForms islands (eForms-DE 1.1: 29, 1.2: 25, 2.0: 73; T01: 9) have exactly one TED twin.
- All 136 pairs agree on title, CPV, subtype, buyer org and dispatch second. That gives a Wilson lower bound of 97.25 % for "an equal BT-701 means the same notice", and no disagreeing pair was observed.
- No twin exists for the 1 eForms-DE 1.2 record with a national legal basis, or for any of the 50 SDK 1.0 national records.

**Joins in the sample:** 136. Every target is a TED island with no DÖE version.

**Extrapolation** (exact Poisson 95 % interval on 136, which is counting noise only)
- Scaled by merged Tenders, 136/12,513 × 243,588 ≈ **2,647, interval [2,221; 3,132]**. This is the preferred figure: twins and merges both come from the same above-threshold eForms channel.
- Scaled by unmerged Tenders, 136/13,656 × 677,443 ≈ 6,747, interval [5,660; 7,981]. This is a high-side sensitivity only.
  - Here 677,443 = 921,031 − 243,588. The 921,031 is `MERGE_SQL`'s count of every DÖE-touching Tender, islands included.
  - The unmerged pool also holds the pre-eForms months (from 2022-12), where no BT-701 twin can exist.
- The matcher's dry run will give the exact population count, which replaces both figures.

### 3b. Matched band: R1
**Rule.** `R1 = org & title normalised equal & same subtype & D7e & NC-without-value & (DL | DPs | (DPh & IR))`, with the one-to-one check.
- `DPs`: dispatch equal to the second, and the DÖE dispatch is not on a full hour.
- `DPh`: dispatch equal, on a full hour. It needs `IR` as well, because hour-grain values collided in 1 of 2 known negatives.

**Measured**
- Recall 449/458 = 98.0 % [96.3; 99.0]. On true twins, 449/449, lower bound 99.15 %.
- FP 0/1,289 (per-pair upper bound 0.30 %). Precision 1.0, Wilson lower bound 99.15 %.
- This holds without G1. Only 3 negatives pass the core, and the dispatch differs on all 3:
  - Bw 567147↔1142490: 10:12:34 against 10:21:07.
  - Hanau 276995→453533: TED notice 24713010 was dispatched 2025-04-11T06:23:41Z (in the skeptic's file) against the DÖE side's 06:41:06. This settles the recall-gap analysis's open PK read without a new request.

**Islands**
- Measured: 13 correct, 0 wrong. The count is low only because the recall-gap analysis had TED dispatch for 12 of the 136 twins.
- With the skeptic's dispatch data, all 136 twins pass the core and agree on dispatch. R1 could refuse only twins dispatched on a full hour that also lack both IR and DL. IR is missing on 19 twins, so R1 would admit **between 117 and 136**. The exact figure was not computed.

**Marginal yield over 3a:** 0 Tenders in the sample.
- 0 joins on the 51 eForms-family records without a twin, and 0 on the 630 sdk-0.1 records.
- Upper 95 % bound for the population: about 72 (scaled by merged) or 183 (scaled by unmerged), from a Poisson upper bound of 3.69 on 0.
- The 300 random sdk-0.1 records alone bound the join rate only to ≤ 1.26 %.

**Why R1 and not the others**
- L4 has 1 false positive without G1, which fails the amendment's "no false merge in the calibration sample".
- L2 is clean but reaches 44.3 % recall and joins only 2 islands, because 134 of 136 PINs carry no deadline.
- R1 is not yet validated on PINs with full dispatch data; see section 6.

## 4. False-merge shapes and the guard each needs

| # | Shape (example) | Which rules it defeats | Guard |
|---|---|---|---|
| 1 | Lots or trades published as separate procedures under one title: Bezirkskliniken 7×7 (42 of 49 pairs wrong); Roggentin (4 procedures, same day); Hanau | A–G (title, CPV, subtype, Δ; C too when a ref is missing) | Require DL or DPs; never title + org alone; one-to-one check |
| 2 | Framework lots changed the same day (Bw 567147↔1142490) | A, B, C, E, G | Dispatch second (DPs) |
| 3 | Repeated modifications of one contract, each under a new BT-04 (Deichverband) | A, B, G | `NC` on internal ref; D7 |
| 4 | Second procedure beside a corrigendum (DFN NACS, Δ 5 d) | Title and CPV rules | `NC` on ref and deadline |
| 5 | Re-published PINs (ZVBN Ammerland Süd, Heinzelmännchenweg) | "nearest date" picks wrong (T1581303) | Refuse when the pair is ambiguous (4 refused); dispatch resolves Ammerland; never pick the nearest |
| 6 | Internal ref is a project id (17 of 1,028 negatives share it) | F, `org & IR` (17 FP) | The ref is a corroborator only |
| 7 | Generic title with a date-coded ref (Stuttgart Gerüstarbeiten); lot count 2 vs 6 (BA REZ NORD) | A, B, G | `NC` on ref and lot count |
| 8 | Fuzzy titles (similarity ≥ 0.95 raises negatives from 12 to 28; Jaccard ≥ 0.8 admits 22) | Any fuzzy rule | Exact normalised title only; fuzzy goes to `possible_duplicate_of` |
| 9 | Same dispatch second, different notice (6 of 1,028; EFRE Biobank) | Dispatch alone | Dispatch only on top of the core; on-the-hour dispatch needs IR |
| 10 | Exact deadline shared by sibling trades (30 of 1,289 negatives; 24 sdk-0.1 pairs) | `org & DL` | DL only on top of the core |
| 11 | PIN against the later contract notice (Löhne, Δ 7) | Rules without a subtype check | Same subtype required |
| 12 | sdk-0.1 national against eForms EU (Lathen, Jaccard 0.89, deadlines 10 min apart) | Name, near title, deadline | No sdk-0.1 matched band yet |
| 13 | sdk-0.1 siblings from one title template (Hessepark vs Eilbektal, deadlines equal to the minute) | Name + deadline + CPV | No sdk-0.1 matched band yet |
| 14 | UUID key collisions: TED 1110706 (Wesel and Белоградчик); TED 42726 (36 versions, 8 buyers); 17 negatives overlap only through other versions | Tender-level buyer lists | Compare buyers named in each notice, not the Tender's accumulated parties; add a buyer-set disagreement check |
| 15 | Island naming: a DÖE island published before its TED twin gives the component the name `island:<doe>`, which creates an issue-278 ghost on non-rebuild runs | The fold, not the matcher | Rank by `(is_island, published_at, publication_id)` |
| 16 | Weld: a matched edge joining two keyed TED Tenders, or a TED notice paired with two DÖE notices | The fold | Refuse such edges; one-to-one at notice level; component-size cap; tally refusals in the Report like `target_refusals` |

## 5. Build plan

**1. Edge ledger** (durable)
- Add it to the canonical SCHEMA beside `legacy_ojs_keys` (canonical.rs:977), so it survives `reset_tender_layer`.
- Key it by notice, never by Tender id. Suggested columns: `a_notice_id`, `b_notice_id` (nullable) or `b_source` + `b_publication_id`, `kind` (declared / matched), `rule`, `evidence` (JSON), `job_id`, `at`.
- Indexes: `a_notice_id`, `b_notice_id`, `(b_source, b_publication_id)`.
- Declared rows are written in `insert_plan_tx` (9585–9606):
  - existing `prev_refs` (OPP-090), now allowed to cross sources;
  - a new `notice_uuid` producer. Resolve the DÖE side with a prefix range on `notices_publication_id_id`, or use BT-701 directly if DÖE stores it (unverified).
- A ledger write or delete sets `notices.projected=0` on both endpoints.

**2. Where the fold reads it** (the ADR-0011 step, canonical.rs:9980–10182)
- Generalise `plan_prev_edge` (DDL at 9337) to carry `b_source` / `b_notice_id` / `kind`.
- In `PREV_EDGE_JOIN_SQL` (1252), replace `n.source = e.a_source` with the edge's own target source. That replacement also fixes today's dropped cross-source OPP-090 links.
- Keep `b.published_at < a.published_at` for declared previous-notice edges only.
- Put every edge kind into the one `edges` vector before `MinUnionFind` (10097–10109).
- Representative rule at 10064–10096: rank `(is_island, published_at, publication_id)`.
- Add the weld guards from section 4.
- Incremental path: add a ledger closure beside `legacy_closure` (project.rs:2043), merged into the touched set at 2287–2305, with a cap and a fallback to the full path. Today ADR-0011 edges apply only on full re-projections (canonical.rs:9996–10000).

**3. Matcher job** (for example `match-tender-links`), modelled on `match-org-identifiers` (supervisor.rs:1443)
- The `rule` parameter takes `notice_uuid` or `r1`.
- **Dry run:** stores the plan and counts and writes no edges.
- **Review:** reads the stored plan before anything is written.
- **Wet run:** writes only the stored dry plan's reviewed pairs (the altid precedent). It aborts unless the plan re-derives to the stated count (the `expect_gaps` parity precedent), and takes a cap (`max_groups`).
- Candidate search uses bounded seeks: buyer org through `tender_version_parties_org_role`, time through `tenders_current_published`.
- Wrap the arm in `Box::pin` (CLAUDE.md's stack-overflow note).

**4. Review campaign** (boundary cases)
- The 26 positives that R1 admits on dispatch alone: 34355, 63241, 76983, 85427, 127787, 175213, 176393, 292579, 319691, 327601, 367280, 385455, 393975, 427198, 428646, 463198, 474902, 567147, 608718, 886852, 1012065, 1045140, 1104520, 1113899, 1142490, 1153069.
- The 17 positives admitted through `DPh & IR`.
- The up to 19 island twins without an internal ref.
- The 4 re-published PINs: 1576759 / 1577989 and 1579943 / 1581303.
- Pairs that pass L5 but not R1.
- The 30 hand reads issue 481's unit 1 asked for. These are not done yet.

**5. `possible_duplicate_of`** (never merged)
- Served for pairs below R1: L5 without a corroborator, L6, L7 and L8.
- Measured precision: L5 0.9933 (LB 0.9805), L6 0.9741 (LB 0.9552), L7 0.9659 (LB 0.9453), L8 0.9284 (LB 0.9021).
- Counted on the weekly report beside "merged with TED", together with the missed-twin count.
- Not for sdk-0.1 yet: name-blocked word overlap ≥ 0.5 finds 8 records, of which only 1 looks like a true twin.

**6. Tests** (run through `ops/check.sh`, with `--features server` for ad-hoc checks)
- One fixture per shape in section 4 that must not merge.
- A BT-701 pair merges.
- Island published first: the keyed member names the component, with no ghost on a non-rebuild run.
- A ledger row added merges on the next incremental fold, and deleting it un-merges on refold.
- A cross-source OPP-090 link resolves.
- Weld-guard refusals are tallied.
- The wet run aborts on parity mismatch.

## 6. Still unmeasured
- **R1 on PINs with full dispatch data:** only the [117; 136] bound exists. The twin-removed test was not run for R1 either. Its lot-count check is what separates Königssee, but that is not confirmed by a run.
- **BT-701 on the 458 keyed positives,** whether DÖE notices store BT-701 in `notice_ids`, and BT-701 collisions beyond the 795 TED candidate notices read.
- **Other months and eras:** month-to-month variance, the 2023-10 → 2025 ramp, and the pre-eForms period, when TED twins were keyed `ojs:`.
- **sdk-0.1, which has 13,465 DÖE-only Tenders a month:**
  - no positives exist;
  - no numeric record shares an org id with TED;
  - 0 equal titles among name-blocked candidates;
  - documents URL: 0 equal and 0 shared procedure ids across 2,637 pairs with a URL on both sides (re-run of `eval16.py`);
  - whether the 330 EU-basis records a month have TED twins at all is unknown and needs hand labels.
- **The issue's unit-1 count** of DÖE notices carrying a TED publication number (OPP-090, DE1 NoticePublicationID / GazetteID) that are not on their twin's Tender. There is no bounded design for that count yet.
- **Wider negatives:** pairs beyond ±30 days, pairs with different org ids (name blocking cannot be scored), and comparison against every TED version of a candidate rather than the paired one.
- **Smaller loose ends:** how common hour-grain dispatch is on islands; the 5 positives whose DÖE `published_at` is ≥ 2 days before dispatch; how 443341 got joined.
- **Precision at population mix.** Sample precision depends on the 458 : 1,289 mix; the per-pair rate and its bound are what carry over.

Files are in `.scratch/tender-db/481-dedup/ (data/, scripts/; working copy was the session scratchpad)`:
- `positives.json`, `negatives.json`, `unmerged.json`
- `calib/results.json`, `recall/fixes.json`, `recall/r1.py`
- `skeptic/disp_neg_ef.json`, `skeptic/uuid_twins_ef.json`, `skeptic/eval15.py`, `skeptic/eval16.py`