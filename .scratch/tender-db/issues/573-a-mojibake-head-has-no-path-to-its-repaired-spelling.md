# 573 — a mojibake (dropped-accent) head has no path to its repaired spelling: every hold that could release it is blocked

Status: needs-triage — filed 2026-10-19 from issue 548 (U1 and U2 routed it to "its own unit"). Next: triage — size the class
(how many held/kept heads carry a dropped-accent or mojibake shape the unfit test misses), then decide between an R0-style
"defective head" path and teaching the unfit test the shape plus a release the wet filters admit.
Kind: data quality (served organization names)
Relates to:
- 548 (U1 found it; U2 left it out of rule v13);
- 524 (the head election and its wet filters: `exclude_released: unfit` stands while v12's re-judge is failed);
- 552 (the `unfit` / lines guard precedent: a control-only head's release was withheld for a reason of its own).

## Verify

    curl -s https://tenders.zebreus.click/v1/organizations/22435268 | jq -r .name

- **done**: prints the repaired spelling (`Comité consultatif interrégional du règlement amiable …` or the cluster's fit spelling),
  or this issue records why the broken head is kept.
- **open (2026-10-19)**: prints `Comit consultatif interrgional du rglement amiable …` (the dropped-accent head).

## What was found (issue 548 U1, plan 2569 and plan 2653)

22435268's head `Comit consultatif interrgional du rglement amiable` has lost every accented letter (a mojibake that dropped the
bytes rather than garbling them). The election wants the right spelling, and four things block it in turn:
- `review_body` (a serve hold) holds it: the broken head no longer matches the review-like stems, so the hold reads a review
  body's self-rename as a non-review head gaining a review phrase.
- If `review_body` were fixed, `head_current` re-holds it: the head was printed 2024-02-09, within 1,095 days.
- If the unfit test learned the dropped-accent shape, the change would ship as an `unfit` junk release, and every wet run passes
  `exclude_released: unfit` (v12's filter, standing while its re-judge is failed).
- So it needs a path of its own: an R0-style rule for a DEFECTIVE head (empty, non-name, and now mojibake/dropped-accent), which
  R0e/R0n already are and which no recency or serve hold asks.

## Proposed units

1. **Size (S).** Over the v13 plan's `held` and `keep:S1/bar` entries, count heads whose folded words are a dropped-accent form
   of a cluster spelling (each word equal after deleting the non-ASCII letters of the other) or carry a mojibake marker the unfit
   test misses. Salted 30 read if > 60.
2. **Decide (S).** R0d (defective head → the top cluster's fit spelling, never held by recency/serve holds, asked of the bind
   guards) vs teaching the unfit test and adding a narrow release the filters admit. Record the bar (0 wrong on the read).
3. **Build** as a new head-election epoch, re-plan, re-judge, wet through 524's runbook.
