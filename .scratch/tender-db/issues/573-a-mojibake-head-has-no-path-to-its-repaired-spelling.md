# 573 — a mojibake (dropped-accent) head has no path to its repaired spelling: every hold that could release it is blocked

Status: **DONE 2026-10-24** — wet 2775 (rule 524u2-v14, org_ids = the 15 R0d heads, exclude_rules R0w,R2) RENAMED 15 of 15 (pre-pass 15 ready, 0 already renamed, 0 drifted, 0 gone; 0 raced; each with an org_name_refresh_log row and an `organization changed` row; undo `{"kind":"revert-org-name-refresh","job":2775}`). Verify read 19:5xZ: `GET /v1/organizations/22435268` → `Comité consultatif interrégional du règlement amiable` (done). Was: ready-for-agent — **U3 apply in progress 2026-10-24 (deploy #3 6cb4408)**: dry plan 2771 (v14) — CHANGES 1,668, **R0d 15 (296 mentions)**, R1 104, R2 1,549; GO PASS per spec 6.3 (port: 15 planner R0d, 0 held, extra [] missing [], every new_name as decided; epoch 524u2-v14, truncated/held_truncated false, undated null, rev = deployed); envelope computed_at 1792865325, scope_ids null (re-read before enqueue, unchanged). Canaries: 2 unexpected (178 keep:bar, want hold:recent) — informational, not part of the GO bar; recorded for 524. **Wet 2775 queued** (org_ids = the 15, exclude_rules [R2,R0w], exclude_released [unfit]) behind 577 slice 1 (2773/2774); no other refresh-org-names until it ends (C1). NEXT: read the applied report (plan_computed_at = 1792865325, applied 15, drifted/gone/raced 0), then Verify and DONE. Was: ready-for-agent — **BUILT 2026-10-24 (b023329 on the session branch): R0d head rule, HEAD_ELECTION_EPOCH 524u2-v14.** Drafted from `build-specs/573.md` (spec + adversarial check, wf_b8e47a6e-d46) in an isolated worktree (wf_5cd7b201-7c5), merged, compiled and focused-tested (91 passed across the deploy #3 batch); full gate and a 3-lens adversarial review (wf_c35791a6-e18) running. NEXT: deploy #3 when 546 U4's project 2765 ends; after deploy #3: the dry re-plan, then the wet run scoped to the R0d ids (spec D573-7), then Verify. Was: ready-for-agent — unit 1 SIZED and unit 2 DECIDED 2026-10-19 (`.scratch/tender-db/573-triage/size.md`, size.py): plan 2653 holds 17 hits — 15 dropped-accent heads (14 head_current + the exhibit 22435268 under review_body; all French courts or committees printed 2024-01-22..02-19, one batch) and 2 R2 mojibake heads — read 16 R / 1 N / 0 W (15/15 R on the dropped-accent arm); a lower bound (keeps are not listed; their uniform samples read 0 of 30). Decided: a narrow R0d (char-level dropped-accent head of another cluster's spelling, unique top passing R1's bars, asked only of the bind guards, no marker arm, R0d accepted by exclude_rules but not in the runbook's filter). Teaching the unfit test ships 0 (exclude_released unfit); waiting ships 14 unread in 2027-01/02 and never the exhibit. NEXT: unit 3 build as a new head-election epoch (low urgency), re-plan expecting exactly these 15 R0d changes. Was: needs-triage — filed 2026-10-19 from issue 548 (U1 and U2 routed it to "its own unit"). Next: triage — size the class
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
- **read 2026-10-24 (after wet 2775)**: prints `Comité consultatif interrégional du règlement amiable` — done.

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


## Unit 1 — sized; Unit 2 — decided (2026-10-19, wf_7d4abf7a-699)

- **Hits (plan 2653, held 11,515 + planned 1,693):** 17. hold:head_current 14 (dropped-accent; 2 of them, 22399787/22399788
  `Pointe--Pitre`, need the char-level test because a whole `à` and its no-break space were dropped), hold:review_body 1 (the
  exhibit), change:R2 2 (22504462 `Ã©`, already `unfit` mojibake; 22678565 U+FFFD, read `control`; both held back by the R2 filter).
- **Read:** 16 R, 1 N (22678565), 0 W; the dropped-accent arm 15/15 R. Pointe-à-Pitre would serve `Pointe-à -Pitre` (a space for
  the no-break space): cosmetic.
- **Exhibit 22435268:** R1, V/T/H 21/18/3, head_newest 2024-02-09; the elected spelling `Comité consultatif interrégional du
  règlement amiable` (18/21). review_body holds it because the broken head folds to no review phrase or stem; fixing that alone
  meets head_current until 2027-02-08, and an unfit release is filtered on wet runs.
- **Unit 2 decision: R0d.** Trigger: the head equals, case-folded with whitespace collapsed, a spelling S of ANOTHER cluster with
  every non-ASCII character deleted, at least one word that keeps an ASCII letter lost a letter, and no deleted word was made only
  of 2+ non-ASCII letters (excludes the 2 Greek-word omissions 11221362, 22751177). Elect S's cluster only when it is the unique
  top and passes R1's bars (T·20 ≥ V·11, T ≥ 3, T ≥ 2H; all 15 pass, tightest 22499321/22499322 at T 4, H 2, V 6); serve its
  display spelling with today's twin rule; ask only the bind guards (as R0n), never the window, residue, serve holds,
  head_current, top_stale or initialism. No marker arm (it would sweep in issue 552's control-only heads). `exclude_rules` accepts
  R0d; the runbook's filters stay ["R2","R0w"]. Bar: 0 wrong on the read (met). Unit 3's re-plan must produce exactly these 15
  R0d changes; any other R0d item is read before a wet run.
