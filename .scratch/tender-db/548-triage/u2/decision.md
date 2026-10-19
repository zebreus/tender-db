# 548 U1b + U2 (2026-10-19)

Input for every number: the stored v12 plan `org-name-refresh-plan-2026-10-16.json.gz` (epoch 524u2-v12, rev 3475272,
as_of 1792191671). Scripts and outputs: `548-triage/u1b/`.

## U1b: the T1 dated 30 (top_stale's T1 release arm)

T1 (triage D4) holds `top_stale` only when `rival_newest > top_newest`. The release arm is the top_stale rows where that
is false.

- Arm size (`u1b/arm.py` writes `arm.json`): **1,318 of 2,425** top_stale holds (54%), 80,037 of 174,559 mentions, 742 with
  H<=1. Every row is R1, with no `recent_released` and no `release_withheld`. top_stale is the last hold, so the whole arm
  would ship.
- Reverts: **0 structurally**. In no arm row was the top printed before the head (`top_newest < head_newest` in 0 of 1,318; strictly later in 1,315, the same day in 3).
- Draw: salt `548-u1b-`, sha256 ascending, 30 rows from a pool of 1,298. The pool excludes gate (d)'s 20 top_stale ids
  (12 of them are in the arm), the 8 arm ids on 524-v11/skip-ids.tsv, and the ids in 524-v12/rejudge-2569 (none are in
  the arm). The draw is in `u1b/sample.json`.
- Evidence: 42 public GETs (`u1b/api/`), at least 2.0 s apart by the log (2.2 s sleep), no 408s.
  - 30 org GETs. In each one the held head is still the live served name with the same identifier, and the mention
    count matches the listing. 4688514 has one new mention since the plan.
  - 8 `name_prefix` lookups and 4 tenders lists for the uncertain rows.
- Verdicts (`u1b/verdicts.py`, `verdicts.json`): **13 R / 15 N / 2 W, 0 reverts. That FAILS the bar of at most 1 wrong.**
  - By head size: H<=1 is 10R/8N/1W; H>=2 is 3R/7N/1W.
  - **W 9168556**: `Usługi Leśne Ryszard Flak` → `Partner konsorcjum Usługi Leśne Ryszard Flak`. The change ADDS a
    consortium-role prefix. The top cluster has 4 of 7 votes from 4 notices, so votes per notice would not stop it.
    No serve hold tests for a role marker being added.
  - **W 17269219**: `Mithieux TP` → `MITHIEUX`. Same firm, but the new name drops `TP` and is all caps. The same SIRET's
    lot orgs and `société mithieux tp` print MITHIEUX TP, and a different firm, MITHIEUX OI, exists. This is a
    same-entity downgrade, labelled the way 524 gate (d) labelled 5638950 Brlic.
  - The N verdicts are mostly shorter or longer forms of the same body (DREAL, TGSS, Enabel, Aspira, APASE, Transrevis).
    Some names are unverified: Carflor, JJ Santo António, and Energiebox (a trade name vs Stichting Trias Innervatie).
  - The R verdicts include real renames: 3333925 BKK Közút → Budapest Közút; 8587172 SAS Lerichemont → Hénéo, where
    `b_8587172` shows the identifier buying as Hénéo in 2024; 11039579, a surname change. They also include role lines
    and junk heads dropped: 4930460, 12782788, and 21495464, whose head is a procedure title.
- Role marker added, counted on the listing (`u1b/role_added.py`): 1 of the 1,318 arm rows (the Flak row itself) and 37
  rows across the held listing (head_current 24, one of them the false positive `Partners`). A role-marker-added serve hold
  would remove exactly this wrong. That rule was shaped on the sample, though, so it needs an out-of-sample read.

## Votes per notice: measured (`u1b/pernotice.py`, two bounded /v1/sql reads q03/q04)

Each read was an IN-list seek on `organization_mentions_org` (the plan was probed locally on plan.db first): 1,124 and 1,631
rows, 1.9 s each. Clusters approximate `head_vote_key`. An org counts as reproduced when the per-mention clusters give
the listing's top, head and voting exactly. That holds for 132 of 138 orgs. 4 of the 7 gb rows miss, because the
approximation does not fold `Limited`/`Ltd`/`LLP`/`Public Ltd Company` the way the real vote key does. The per-notice bar is R1's bar, counted in notices: top >= 3, top >= 55% of votes, top >= 2H, and a
unique top.

| set (reproduced) | blocked per notice | verdicts of the blocked |
|---|---|---|
| gb_family ship 7 (3) | 2 | 2 R (Bethell, EDF) |
| family H<=1 ship 22 (22) | 0 | none |
| consortium subset ship 79 (78) | 8 | 4 R, 4 unread |
| T1 sample 30 (29) | 3 | 2 R, 1 N; **neither W** |

It stops U1's 2 wrongs: Barameda (q02: 7 mentions, 1 notice) and CPR (3 mentions, 2 notices). Both are already outside
the adopted releases: Barameda has H>=2, and CPR fails the subset test. On the release sets it blocks only rights.
**Not adopted.** Keep it as a candidate if a future release set has lot-repeat wrongs.

## U2: the release decision, per hold

| hold | decision | reason |
|---|---|---|
| **gb_family** | **RELEASE at H<=1 (D3's stray head), not whole** | 7/7 shippable read right (5 confirmed by the CH register). The narrow form ships 5 of them. The 2 rights it keeps held, 13448374 (H=7) and 9087207 (H=2), are the whole H>=2 evidence, so n=2. In the sibling guard family, the H>=2 class is 7R/4W and holds all 4 of its wrongs. One predicate (head votes <= 1) for both family guards gives U3 one rule and one test shape, and it bounds what the weekly upkeep (524 u2c) can ship unread. Cost: 2 right changes stay held, and no wrong name is served. Releasing the guard whole would stop it firing on every future H>=2 row on the evidence of 2 rows. |
| **family** | **RELEASE at H<=1** | 22/22 R (462 mentions). All 4 wrongs are H>=2. |
| **consortium_removed** | **RELEASE on the token-subset test** (u1c `subset.py`; legal forms = crosswalk `family_token` keys plus `R2_LEGAL_FORM_TOKENS`, not the wider list) | Salted 30: 29R/1N/0W. 34 of the 79 now have recorded verdicts (33R/1N). CPR does not match. U1c's caveat (74 matched rows behind head_current/top_stale are unread) does not bite, because both of those guards stay. |
| organ, code_added, address_added, review_body, caps_variant, doubled, no_fit, seat_dropped | **KEEP** | Their reads are 0-2 R of 10. seat_dropped already holds the generic-name shape (5350675). Serving the doubled half is not adopted: it differs from the head in 30 of 80 rows, mostly neutral Basque↔Spanish swaps, with 1 known harmful case (5857542). |
| head_current | **KEEP** | Dated 30: 20R/7N/3W. That fails the 1-wrong bar, and the 3 wrongs are 3 different shapes. |
| top_stale | **KEEP; T1 NOT adopted** | U1b: 13R/15N/2W fails the bar. The candidate "T1 + a role-marker-added serve hold" passes only in-sample (1 W), so it needs a fresh dated 30 that excludes these 30, gate (d)'s 20 and the skip ids. |
| nhs_check, placeholder_id | **KEEP** | Neither ships anything: head_current re-holds every row. |
| R2 / initialism | **R2 stays planner-only (D1)**; initialism is moot | No R2 sub-shape separates its rights from its wrongs. |
| mojibake 22435268 | **Its own unit** (an R0-style path for defective heads) | head_current re-holds it (its head was printed 2024-02-09), and an `unfit` release is excluded on wet runs, so neither a review_body fix nor an unfit fix ships it. |
| votes per notice | **Measured, not adopted** | See the table above. |

## U3: predicted by_hold moves (full listing; `u1b/u3_predict.py`, `u3_predict.json`)

A release takes every matching row out of its reason, so each prediction is for the full listing. The table also shows
where each row lands, using U1's `cut()`.

| release | by_hold drop (orgs / mentions) | lands: ships (R1 change) / head_current / top_stale / R2 change |
|---|---|---|
| gb_family H<=1 (adopted) | **−7** / −86 (10 → 3) | 5 / 1 / 1 / 0 |
| (gb_family whole, not adopted) | (−10 / −209) | (7 / 2 / 1 / 0) |
| family H<=1 | **−36** / −680 (79 → 43) | 22 / 7 / 4 / 3 |
| consortium subset | **−179** / −2,769 (246 → 67) | 79 / 56 / 18 / 26 |
| **net, adopted** | gb_family −7, family −36, consortium_removed −179, **head_current +64, top_stale +23** | holds 11,652 → **11,517** (−135); R1 changes +106 (upper bound); R2 changes +29 (excluded on wet runs) |

- The ±10% check has to include the head_current (+64) and top_stale (+23) increases, not only the 3 drops. The ship
  counts are upper bounds, because the serve holds run after the bind guards and the listing does not model them.
- The task's −10 for gb_family is the whole release. Under the H<=1 release adopted here the drop is −7. If U3 ships the
  whole release instead, use −10 and +65 head_current; the predictions change by those rows only.
- **Cross-guard, 15783942** (`Consortium consisting of Kontron …` → `Kontron Transportation GmbH`, H=1). It is in family's
  −36. If family ships without the consortium release, it moves to consortium_removed (+1) and that reason drops −178
  net. With both releases it matches the subset test and lands in top_stale (it is one of family's 4). It never ships
  either way. No other family-H<=1 or gb_family row crosses.
- U3's unit tests: gb 10213161/11628199 released (H=1); 13448374 and 9087207 still held (H>=2). family H<=1 released and
  an H>=2 row held (3728305). consortium 8217358 released and 18287356 CPR held. The cross-guard row 15783942. Compute
  the gb ship count; do not hard-code it. Port the fold with `fold_diacritics`, not NFKD.

## Budget

- 42 public GETs, tagged `(u1b)` in `../api_requests.log`, at least 2.0 s apart by the log, no 408s.
- 2 of 12 /v1/sql requests: q03 and q04, both plan-probed locally first and logged in `../sql_requests.log`.
- No admin jobs, no writes on the box, no code edits, no git.
