# 519 U3: adjudication and the rehome batch (2026-10-12)

## Rehome challenge (wf_6487ead6-224)

- An independent challenger re-read the 25 other-body rows from the notice evidence (cached `measure/m7-raw/` plus
  live public reads and primary-key SELECTs). It agreed with M7's target at HIGH on all 25: every target was live and
  every mention still stood on its case org. `u3-challenge.json`.
- On 46991199, two independent readers (reviewer and challenger) found FTS 036139-2022 amendment 2. It corrects I.1
  Name from "NHS England North East and Yorkshire" to NENC's name, so the mention is NENC's own, not NHS England's.
  Both were HIGH on QHM survivor 31556600. Decision §1.4 was amended for a published correction naming P (see the end
  of `../unit2-decision.md`), so ruling 3 takes its first branch.

## Posted and applied

- **POST** `/admin/rehoming` cohort `rehoming-519-2026-10-12`: 26 recorded (`rehoming-519.json`).
- **Dry** `apply-rehoming` (job 2338): 195 pending, 26 eligible, 26 moves, 4 missing_target (the PMO verdicts, as
  M3 predicted). The plan's tuples equal the 26 posted, exactly (`rehoming-plan-body.json`).
- **Wet** (job 2339): 26 mentions moved; 30 party rows and 7 bid-party rows followed them across 25 tenders; 25
  notices were re-queued.
- `project` (job 2340) re-derived those 25 notices: 25 corrected. The first live `[issue 519]` lines read `ARMED`
  with 0 triples, so the production child gate is wired.

## Re-census (job 2341; `census-2341-body.json`)

- The six mixed holders are now clean (`denied []`, `under_review: true`): 13T, 26A, 15E, 15C, 15N and 93C.
- The 10 rehome targets now appear as child holders under the parent code, because each carries a rehomed
  child-coded mention. Every one is denied: `child-carries-ppon`, `child-triple-shape`, `minted-identity` or
  `child-name-refused`. They are non-members and are held by omission.
- Child holders went from 58 to 68 and their mentions from 1,804 to 2,703. The added mentions are the targets'
  whole mention counts.
- 171 codes stay applied, 0 keys regrown, 0 identity strays.

## Cohort candidates (`u3-cohort-candidates.json`)

- 23 PPON-survivor parents, 34 members, 1,475 mentions. That is exactly the measured phase-A fold. 28 members are
  sub-ICB or zero-only, under QHM ×8, QOQ ×3, QWO ×3 and 14 single-member codes; 6 are ruling-7 sites (RBT20, RCU51,
  RCX70, B4Z5Z, Q7N9Z, RYY13).
- Out of the cohort:
  - 31615636 (00P "Cumbira", refused);
  - 03Q (refused) and 04Y;
  - every anchor-parent child;
  - X7V4Q, RTR54, RNZ91 and RXX1Y (sites, refused);
  - the rehome targets.
- S1 panel: wf_8d58c151-b93, with a reviewer and a challenger per group (QHM, big sub-ICB, small sub-ICB, sites).

## S1 panel (wf_8d58c151-b93; `u3-panel.json`)

- Every group had a reviewer and a challenger reading independently: QHM, the big sub-ICB bodies, the small sub-ICB
  bodies and the sites.
- 33 of the 34 candidates are IN at HIGH from both.
- B4Z5Z (The Royal Marsden Hospital under RPY, the reviewer's call): the reviewer read it HIGH and the challenger
  MEDIUM. Including it would rest on calling a site name a trust name, and the same-named provisional rows 31607240
  and 21469072 would stay apart. S1 needs both at HIGH, so it is held by omission and RPY gets no verdict.

## The cohort and the run (U5, runbook steps 4-8)

- **Posted** `nhs-body-children-2026-10-12`: 22 verdicts, 33 members, 1,474 mentions
  (`verdicts-nhs-body-children-2026-10-12.json`). Each verdict's `reviewed_holders` is the live holder set of
  census 2341.
- **Dry** 2342: 22 admitted.
- **Capped wet** 2343 (`max_groups` 5, in code order QGH, QHL, QHM, QJK, QMJ): 12 rows removed, 601 mentions, held 0;
  ledger 273 → 285.
- **S3** (`s3check_children.py` against dry 2344, `s3-read-capped.json`): 0 FAILS. Every loser answers 308. All 11
  child codes (`00L 00P 01H 13T 16C 84H 99C 15E 15N 93C 18C`) and every parent code answer exactly the survivor.
  Survivor mentions equal the members' sum exactly, and non-members are unchanged.
- **Canary** 2345 (28 notices: oldest, middle and newest per child code, rehomed mentions excluded, plus the QWU
  trio): AGREES 89/89; triple arm asked 22, bound 22.
- **S6 dry scoped unwind** 2346 (`code QHM`, `cohort nhs-body-children-2026-10-12`): 508 mentions move onto 8
  restored rows (3 by name through 234, 505 by identity through E0); `code_kept: true`; nothing of 513's moves.
  - Edge: the 13T identity group is 489, not 488. The unwind picks mentions by literal, so it would also take back
    the rehomed 46991199, which still carries `GB-NHS-13T`. It was 13T's mention before the rehome, so this is no
    wrong merge, but the unwind is not rehome-aware. Re-apply that one rehome after any such unwind.
- **Full wet** 2347: the other 17 bodies; 21 rows removed, 873 mentions, 8 winners repointed, held 0, residual 0.
- **S3** (against closing dry 2348, `s3-read-full.json`): 0 FAILS. The script now skips a dual-parent child that
  folded under its other parent: D4U1Y 31535814 was a holder under QNQ and QRL and folded into QNQ, its chosen
  parent; it answers 308.
- **Canary** 2350 (30 notices: B2M3M, 26A including the lower-case `26a`, and D4U1Y three each; post-April 46810637
  and 47366322; one per other child code and site): AGREES 92/92; triple arm asked 35, bound 35.
- **Closing dry** 2348: `admitted_codes` [], `nhs_body_regrown` [], 0 identity-stray, ledger 306 (273 + 33), 171
  applied. Child holders 68 → 35 (1,304 mentions).

## What stays out (by omission)

- **Anchor-parent children** (ruling 5): 13 holders, about 290 mentions.
- **Digit and five-character parents** (U6, a separate decision).
- **B4Z5Z.**
- **Refused holders:**
  - 00P 31615636 ("Cumbira");
  - 03Q (NHS England NEY);
  - 04Y;
  - W2U3Z;
  - X7V4Q, RTR54 (NENC under trust site codes; optional 317 to 31556600);
  - RNZ91 (Salisbury Medical Practice);
  - RXX1Y (Mind Matters).
- **The 10 rehome targets**, which hold their rehomed child-coded mentions.
- **M6's 452 cohort:** QHL15E, the COH and PPON values under GB-NHS, the NHS→NHS typos and the junk values. It is
  independent of this fold.
