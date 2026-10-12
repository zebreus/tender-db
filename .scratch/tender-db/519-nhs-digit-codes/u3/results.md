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
