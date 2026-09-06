# 362 — merge verdicts: the execution path for the groups the R2 name gate leaves standing

Status: DEPLOYED 4489cd5 (2026-09-06 04:1x UTC), CAMPAIGN RUNNING — store, planner path (keep denies; HIGH merge with member-set parity admits; wet stamps), POST /admin/merge-verdicts and GET /admin/case-reviews?table=merge live and verified empty; the sonnet review of the 451 groups (13 batches + 3 blind samples, workflow wf_7799e1cb-bfc) started 04:0x. NEXT: post.py → POST → match-org-identifiers dry (read admitted_verdict/denied_verdict) → wet → project → record. Was: BUILDING 2026-09-06 04:0x UTC.
Kind: capability (organization layer merge machinery; the 311 review loop's execution path for merges)
Relates to: 359 (the name gate that creates the queue), 355 (the country-verdict store this mirrors), 311 (the loop), 329 (E0: the same path admits its reviewed groups), 300 Stage 2 (R2)

## The gap

Issue 359's name gate denies an R2 group whose named members share no core token: 451
groups after the refinement (`.scratch/tender-db/359-denied-names-2026-09-06.json`,
job 759). Read by hand, about half are the buyer's-identifier-on-the-winner's-row error
the gate exists for, and about half are ONE entity the rule cannot see: a rename
(Dimension Data → NTT, Tönsmeier → PreZero, Alibus → Nomago), an acronym (PGK, ISVEC,
KOK, PUK LE MO), a translation (BULiGL, PISM), a typo (KrakTansRem, BIEGOSGERA), a
spacing (Lore star). A reviewer can tell them apart in seconds; nothing could execute
the answer — R2 would deny the same group every week, and 311 already noted "there is
no execution path for a merge; record only".

## The path

`org_merge_verdicts` — one row per (country, scheme, key, cohort): the R2 group exactly
as the planner names it, `members` the ascending org-id set the reviewer READ, `action`
merge|keep, confidence, rationale, applied stamps. Recorded through
`POST /admin/merge-verdicts`, read back through `GET /admin/case-reviews?table=merge`.

The R2 planner consults it per group, before the name rule (step 4a):
- `keep` (any cohort, any confidence) → the group is denied — `denied_verdict` — for good;
- HIGH `merge` whose reviewed member set equals the live group after the consortium
  exclusion → admitted past the name rule — `admitted_verdict` — and every other denial
  (the VAT-group wall) still applies; the wet merge stamps the verdict with the survivor
  and the job id in the same transaction;
- anything else (medium/low, already applied, a member joined or left since the review)
  → `verdict_stale`, and the rules decide as if no verdict existed.

Member-set parity is the T4 rule at the smallest grain: a reviewer's "these two are one
company" must not fuse a third row that arrived under the key afterwards.

E0 (issue 329) runs through the same planner, so a reviewed E0 group is admitted the same
way — the review path 329's wet run was waiting for, without changing E0's own rule.

## The campaign (this firing, after the deploy)

The 451 groups, each with its members' names, literal identifiers and mention counts,
reviewed by a sonnet reviewer + adversarial challenger per 35-group batch under a
same-entity rubric (merge: rename / acronym / translation / typo / spacing / parent and
its branch or subsidiary sharing the register number; keep: two distinct organizations,
the buyer's number on the winner's row, a consortium of several members, a person and an
institution). Only HIGH merges the challenger does not dispute are recorded HIGH;
everything else is recorded medium (never applied) or `keep`. Then `match-org-identifiers`
dry (`admitted_verdict` = the HIGH merges) → wet → project. Tooling under
`.scratch/tender-db/362-campaign/`.
