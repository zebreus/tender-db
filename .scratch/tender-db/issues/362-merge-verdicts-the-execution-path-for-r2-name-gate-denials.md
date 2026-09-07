# 362 — merge verdicts: the execution path for the groups the R2 name gate leaves standing

Status: DONE — campaign 2 run 2026-09-07 (owner): the 170 groups the name gate held after the 358/363 folds reviewed (cohort `r2-denied-2026-09-07`, 139 verdicts: 65 merge/high applied by R2 job 794 — 65 groups, 84 rows removed, 858 mentions, 699 winners repointed — 56 merge/medium standing, 18 keeps); the gate's queue is 87 (needs-more-evidence and disputed). Campaign 1: DONE 2026-09-06 05:3x UTC — path deployed (4489cd5) and the 451-group review executed through it: 403 verdicts recorded (172 merge/high, 57 merge/medium, 153 keep/high, 21 keep/medium); R2 dry (job 764) admitted all 172 HIGH merges (0 stale), denied 174 by verdict and 105 by the name rule; wet job 765 merged 169 groups (191 rows, 3,338 mentions, 8,463 parties, 1,292 winners, 1,713 tenders), stamping each verdict; the 3 admitted-but-unmerged groups (RO:cui 14838148 / 2779625 / 4267117, multi-member) were stopped by a later denial in the stack. Record: `362-merge-verdicts-2026-09-06.json`. Was: DEPLOYED, CAMPAIGN RUNNING.
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

**Record-versus-reality probe (2026-09-06 21:xx UTC), two verdict-admitted merges.**
`IT:piva/00110410198` (SAMEC SPA 4 mentions + S.A.M.E.C. SpA 1) and `IT:piva/00125230219`
(Wurth 2 + Wuerth 1 + Würth 1). The verdict store (`GET /admin/case-reviews?table=merge`)
stamps both `applied_at` 1788672628 by job 765: "merged 1 row(s) into 17127676" and
"merged 2 row(s) into 10001015". Reality, by primary-key and index seeks through
`/v1/sql`: exactly those two rows stand in `organizations`, the three losers are gone,
and the survivors hold 5 and 4 mentions — the members' counts summed, none left on a
loser. `org_merge_log` itself is an operator table off the public allow-list (correctly:
"not in the queryable public surface") and has no admin read route, so the verdict
store's stamps are the audit record for these; the ledger is reachable only on the box.

## Queue growth from the 2026-09-07 folds (owner note)

Issue 358's country moves and issue 363's label repair reunited pairs whose NAMES differ, so
R2's name-gate denials rose 105 → 142 → 170 across the two dry scans (jobs 785, 790). Two shapes
sit in the new 65: departments sharing one register id (`Turun kaupunki, joukkoliikennetoimisto`
beside `…Kiinteistöliikelaitos`; `Ville de Baillif` beside `Caisse des écoles`), which a reviewer
should mostly KEEP, and acronym/expansion pairs (`SEMAVIL — SAEML` / `Soc mixte aménag Ville
Lamentin`, `NPEI` / `Nalem peinture étanchéité isolation`), which are the merge shape. The
listing is complete under the cap in the stored `r2-merge-plan` (`denied_names_listing`); a
second review campaign over these ~65 groups is this issue's next unit, same rubric and path.

## Campaign 2, run (2026-09-07 09:0x–10:1x UTC, 11 agents, ~1.0M tokens, 37 min)

Input: the 170 groups in the dry scan 790's `denied_names_listing` (PL 92, FR 35, FI 30, IT 12,
CZ 1) — the 105 campaign-1 residue plus 65 pairs the 358 country moves and 363 label repair
reunited. Tooling: `enrich.py` (on the box: provisional flag and mention count per member by
bounded PK/index reads) and `split.py` (5 round-robin batches of 34 over the country-sorted
list, a blind sample of every 9th case) — the two scripts campaign 1 ran ad hoc; `review.js`,
`rubric.md` and `post.py` unchanged. Sonnet reviewers and challengers, session-model blind sample.

Reviewer: merge/high 93, merge/medium 28, keep/high 13, keep/medium 9, needs-more-evidence 27.
Challenger agreed on 124, disputed 46 (23 merge/high → recorded medium). Blind sample: 14 of 19
same verdict. Hand-park (the campaign-1 KIO bar — a HIGH merge that fuses DISTINCT institutions
or SIBLING units with ≥20 mentions on the smaller side is a visible misattribution, parked at
medium): the six-member KIO/UZP group (again), the KIO/UZP REGON pair, TAR Milano with TAR
Veneto, Sisäministeriö with Ålands polismyndighet, and a Warsaw district sports centre with a
Warsaw primary school (1,618 and 281 mentions). Recorded 139: **65 merge/high, 56 merge/medium,
11 keep/high, 7 keep/medium**.

R2 dry (793): groups ≥2 1,343 → 1,017 (the 363 fold had run), `verdict-merge` 68 (65 + the 3
standing Romanian groups denied downstream), `verdict-keep` 192, `denied_names` **170 → 87**,
plan 65. Wet (794): **65 groups merged, 84 org rows removed, 858 mentions, 1,222 parties, 394
bid-parties, 699 winners repointed, 609 tenders touched**; `project` (795) nothing to rewrite.
Verdict store after (`GET /admin/case-reviews?table=merge&cohort=r2-denied-2026-09-07`): 65
merge/high stamped applied, 56 merge/medium and 18 keeps standing — exact.

What the HIGH merges were: renames (Liikennevirasto → Väylävirasto, Pöyry CM → Ramboll CM, Lima
Polska → Enovis Poland, Perlan Technologies → Altium International, VR Kunnossapito → VR
FleetCare, HUS → HUS Group, Plastic Omnium Caraïbes → Sulo Caraïbes, CA Sud Basse-Terre → CA
Grand Sud Caraïbe), acronyms (SRR, CANGT, ATM-OI, JW 3964 / Wojskowe Centrum Edukacji
Obywatelskiej), spacing and case (LEASE CAR, sm geag, TK Biotech, Ekokem), a city and its
units under the city's number (Helsinki and its housing office; Oulu and its rescue enterprise;
Ville du François and its procurement department), and consultancies' twin rows (Safege / Suez
Consulting).

The 87 the gate still holds are the needs-more-evidence (27) and challenger-disputed groups —
names alone cannot tell; a third pass would need register-history evidence (PRH, KRS, INSEE),
which is a different tool, not another read. Records: `362-campaign/post-body-2026-09-07.json`,
`campaign-2026-09-07.json` (joined reviews, challenges, sample).
