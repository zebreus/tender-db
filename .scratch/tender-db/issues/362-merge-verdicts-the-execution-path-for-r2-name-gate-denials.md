# 362 — merge verdicts: the execution path for the groups the R2 name gate leaves standing

Status: DONE 2026-09-06 05:3x UTC — path deployed (4489cd5) and the 451-group review executed through it: 403 verdicts recorded (172 merge/high, 57 merge/medium, 153 keep/high, 21 keep/medium); R2 dry (job 764) admitted all 172 HIGH merges (0 stale), denied 174 by verdict and 105 by the name rule; wet job 765 merged 169 groups (191 rows, 3,338 mentions, 8,463 parties, 1,292 winners, 1,713 tenders), stamping each verdict; the 3 admitted-but-unmerged groups (RO:cui 14838148 / 2779625 / 4267117, multi-member) were stopped by a later denial in the stack. Record: `362-merge-verdicts-2026-09-06.json`. Was: DEPLOYED, CAMPAIGN RUNNING.
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

## The campaign, run (2026-09-06 04:0x–05:3x UTC, 29 agents, ~2.3M tokens, 77 min)

451 groups, 13 review batches (sonnet) + 13 challengers + 3 blind samples (session model).
Reviewer: merge/high 199, merge/medium 30, keep/high 156, keep/medium 23, needs-more-evidence 43.
Challenger agreed on 391, disputed 60 (26 of them merge/high → recorded medium: the
external-history renames it could not see in the text — Enea Logistyka/BHU, Eden Springs/
Culligan, Suez/PreZero — and the KIO rows under numbers that are not KIO's). Blind readers:
41 of 51 same verdict. Hand adjustment: the six-member Krajowa Izba Odwoławcza group parked
at medium — the Urząd Zamówień Publicznych department shares the NIP but is a distinct
institution, and fusing its 150 procurements into KIO would be a visible misattribution.

Recorded 403; R2 dry 764: `admitted_verdict` 172, `denied_verdict` 174, `verdict_stale` 57
(the medium merges, by design), `denied_names` 105 (the needs-more-evidence and disputed
groups the gate keeps holding); wet 765: 169 groups merged, 191 rows removed, 3,338 mentions,
8,463 parties, 813 bid-parties, 1,292 winners repointed, 1,713 tenders touched; projection
766 `0 notices`. Feed since the last fold: 169 organization changes, 191 removals, 1,732
tender changes — exact. Verdict store after: 169 merge/high applied, 3 merge/high standing
(the Romanian multi-member groups, denied downstream), 57 merge/medium, 174 keeps.

What the surviving HIGH merges were: renames (Gothaer→Wiener, EDF Rybnik→PGE Energia
Ciepła, Sigma-Aldrich→Merck Life Science, Tractebel→Antea, Rivoira→Nippon Gases, Dimension
Data→NTT, Novabase IMS→Axianseu, Sputnik Software→Nefeni, Południowy Koncern Węglowy→Tauron
Wydobycie), acronyms (PGK, DTŚ, PGF, 43WOG, KOK, MPEC, ZURS, HMS, CIVIS, SOGET, GEFIL, ICES),
translations (BULiGL, PISM, PANSA, UWM, Ministero della Giustizia), typos and spacings
(Consulronix, Spektromtria, KrakTansRem, BIEGOSGERA, C-FORST, Q4 Net, Lore star, McART), and
units of a public body publishing under its number (a gmina and its school, sports centre,
library or road board; a region and its directorate; a hospital and its renamed successor).
