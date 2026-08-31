# 314 — Stage 4's 1.5M candidate edges have no consumer

Status: SIZED, and the proposed first cohort INSPECTED 2026-08-31 — it is
heterogeneous and not review-ready as specified. See the read below
Kind: capability (organization layer)
Relates to: 300 (Stage 4 built it), 311 (was meant to consume it), 312

## The gap

`org_candidate_edges` holds **1,498,485** E3 edges, refreshed by the weekly
wet scan. Nothing reads them. Verified 2026-08-30: the only code touching
the table outside its own writer is a doc comment.

Stage 4 was justified by feeding the issue-311 per-case review loop — the
design's own words, and the reason the store is advisory-only (edges never
merge). Then the 311 campaign ran off the **Bietergemeinschaft cohort**
instead, because that cohort was already enumerable and the edges were not
yet built. The campaign succeeded (823 cases reviewed, 442 strips, 280 of
them since restored per 312) and the edge store quietly became a thing that
is maintained but not used.

## What consuming them means

1. **Cohort enumeration from edges.** A case = an edge (or a connected
   component of edges) whose two orgs are candidates for the same entity.
   The 311 machinery takes verdicts keyed by org id, so an edge-derived
   cohort needs a component→case mapping and a cohort name.
2. **Evidence assembly per case**: both orgs' rows, satellite names,
   mentions, collision counts, PLUS the edge's own `evidence` JSON (which
   already names the reaching key, the group size and each side's witness).
   The batch-shape-v2 enrichment pass generalizes: it is the same bulk SQL
   with a different seed set.
3. **Verdict vocabulary is different from the consortium cohort's.** These
   are same-name/cross-language pairs, so the verdicts are merge /
   distinct-entities / needs-more-evidence — NOT the consortium-vehicle
   classes. New rubric, new gold exemplars, its own pilot before any batch.
4. **Nothing auto-applies at first.** The 311 apply job's safe subset is an
   identifier strip; a merge verdict must execute through the merge arms
   (which re-check their own denial stacks), and that path does not exist
   for review-driven merges. Pilot verdicts land as records only.

## Sizing before building (measure first, per 312's lesson)

1,498,485 edges is far too many to review case by case at ~5.3k tokens per
case. Before any campaign, measure the shape: component-size distribution,
how many components are canonical×canonical (the only ones a merge verdict
could act on), how many are already merged-away duplicates, and how the
e3-xlang slice (57,808) differs from e3-name. A tractable first cohort is
probably "xlang pairs where both sides are canonical and the countries
differ" — small, high-value, and exactly the contamination class the
exemplar (org 23294544) represents.

## Do not do

Do not wire an automatic merge off these edges. They are E3 name-equality
only; §1's tier table puts name evidence in candidate generation, never in
the merge decision. The consumer is review, and the reviewer is the gate.

## STEP 1 DONE (2026-08-30): the store is sized — `org-edge-census` (c35532a)

A read-only census job (the table is outside the public SQL surface, so a
job is the only way to measure it). Prod job 492:

**1,498,485 edges → 272,311 COMPONENTS.** That factor of 5.5 is the whole
point of measuring: a review case is a connected component, not an edge, and
a chain of three orgs is one case rather than two.

| slice | components |
|---|---|
| size 2 | 128,241 |
| size 3-5 | 93,004 |
| size 6-10 | 32,337 |
| size 11-50 | 18,717 |
| size 51+ | 12 (max 158) |
| **canonical-only** (a merge verdict could act) | **24,929** |
| mixed canonical+provisional | 247,356 |
| provisional-only (never actionable) | 26 |
| spanning >1 KNOWN country | 10,415 |
| holding a country-less member | 85,329 |
| dangling endpoints | 0 |

**A correction inside this step.** The first census reported 90,618
"multi-country" components, led by ??-FR (20,331) and ??-DE (12,374) —
where `??` was the census's own label for a country-less row. It was
counting the R3 pool's overlap, not cross-border cases. Fixed and re-run:
the real cross-border number is **10,415**, an order of magnitude smaller
than the figure I would otherwise have scoped a cohort against.

The corrected pair table is itself the validity check — BE-FR 493, AT-DE
466, FR-RE 450, GB-IE 422, BE-DE 360, FI-SE 301, CZ-SK 229, FR-MQ 182,
FR-GP 147: shared-language borders and French overseas territories, i.e.
precisely where one entity legitimately publishes under two country codes.
Nothing random-looking in the top 12.

## What this says about a campaign

At the issue-311 campaign's measured ~5.3k tokens/case, the 24,929
canonical-only components alone are ~130M tokens. Too big as a first bite,
and mostly punctuation/case variants of one name (the sample is full of
"ČEZ ESCO, a.s" vs "ČEZ ESCO, a.s.", "Kolping Berufsbildungs-gGmbH" vs
"Kolping-Berufsbildungs-gGmbH") — high-value but low-ambiguity work that
may not need a per-case agent at all.

**Recommended first cohort: canonical-only AND cross-border.** Both halves
are measured (24,929 and 10,415) but their INTERSECTION is not yet — that
counter is the one number still missing, and it is a two-line addition to
the census. Expect it to be small (low thousands at most), it is the slice
the contamination exemplar belongs to, and cross-border pairs are where a
wrong merge would be most damaging — which is exactly why they deserve
individual review rather than a rule.


## STEP 1 COMPLETE (2026-08-30): the cohort is 962 components

`org-edge-census` now reports the intersection the sizing was missing:

    1,498,485 edges over 1,121,728 orgs (0 dangling)
    272,311 components, max 158
    24,929 canonical-only | 247,356 mixed | 26 provisional-only
    10,415 span >1 KNOWN country | 85,329 hold a country-less member
    COHORT (canonical-only AND cross-border): 962

962 is a campaign that fits: at the batch-v2 rate (~5.3k tokens/case) it is
about 5M tokens, an evening's work, not the 130M a full 24,929-component
sweep would cost.

The cohort's borders are language borders and shared registers, which is
what a real cross-border duplicate class should look like: CZ-SK 68,
FI-SE 64, LT-LV 27, BE-CH 25, NO-SE 24, CH-DE 23, GB-IE 22, AT-DE 20,
BE-FR 20, DK-NO 20.

## …but read the SAMPLE before spending the budget (issue 319)

The census now carries the first 25 cohort components, and reading them
changed what this campaign is. Real duplicates are in there —
`CZ:SARSTEDT spol. s r.o. || SK:Sarstedt spol. s r.o.`,
`ES:Howden Iberia S.A.U || NL:HOWDEN IBERIA S.A.U.`,
`DK/LT/NO:Mercell Holding ASA` — but so is a whole contaminating class:

- `GL:Nukissiorfiit || GRL:Nukissiorfiit` — one Greenlandic utility under
  an alpha-2 and an alpha-3 code. Not a border at all.
- `CH:Gemeinde Glattfelden || EE:Gemeinde Glattfelden` and
  `DE:Vergabekammer Rheinland-Pfalz (×5) || EE:…` — a Swiss municipality
  and a German public body with an Estonian country code.
- `BG-VU 19` in the pair table — Vanuatu, holding Bulgarian entities.

That is **issue 319**: the country column carries alpha-3 codes (151 values,
1,347 rows) and a separate class of well-formed but wrong codes (VU 110,
GY 33). Some fraction of the 962 is that bug wearing a cross-border
costume.

**So the order is fixed: 319's normalization and backfill run FIRST, then
the census re-runs.** The drop in the cohort number IS the measurement of
how much of the signal was the bug — and the campaign then spends its
budget on judgement calls rather than on rediscovering a normalization gap.

## The apply-path constraint, stated before the campaign starts

Verdicts on this cohort will be MERGE decisions, and nothing can execute
them today: `apply-case-reviews` only strips identifiers, and
`org_candidate_edges.state = 'approved'` is a reserved column with no
consumer. So the campaign's first run produces recorded, auditable, INERT
verdicts — which is a fine deliverable — and a review-gated merge arm is a
separate unit that writes entity references and therefore needs its own
dry-first plan and its own panel round (the same bar as issue 317 Unit A).


## COHORT AFTER THE 319 FOLD: 939 (2026-08-30)

The country normalization landed and the census re-ran: 962 → **939**
components, cross-border 10,415 → 10,319. The alpha-3 split was 2.4% of the
cohort, not the large fraction the sample suggested — see 319 for why that
prediction was over-read.

**So the campaign is unblocked and its input is ~939 components**, of which
a known ~39 are still fake borders from upstream country errors (`BG-VU` 20,
`CH-EE` 19 — TED publishing a wrong code, unreachable by normalization). A
reviewer meeting one of those should mark it as a country-data error rather
than a merge decision, and the verdict schema needs that option.


## THE SIZING EXISTS (org-edge-census, read 2026-08-31)

    edges 1,498,485   components 272,311   orgs_touched 1,121,728
    e3_name 1,440,677 · e3_xlang 57,808 · max_component 158 · dangling 0

    canonical_only     24,929      <- the only ones a merge verdict could act on
    mixed             247,356      <- canonical x provisional, most of the mass
    provisional_only       26
    null_country       85,329
    multi_country      10,319
    canonical_cross_border  939    <- this issue's proposed first cohort

    size 2: 128,241 | 3-5: 93,004 | 6-10: 32,337 | 11-50: 18,717 | 51+: 12

## The proposed first cohort is NOT review-ready

This issue proposes "xlang pairs where both sides are canonical and the
countries differ" as tractable and high-value. It is 939 components, which is
tractable. Reading the census's own 25-component sample, it is **at least four
classes with four different correct answers**:

1. **Genuine multinational duplicates** — `DK/LT/NO: Mercell Holding ASA`,
   one company under three country codes. A merge candidate.
2. **Corporate siblings that must NOT merge** — `AT: Steelco Belimed GmbH` vs
   `DE: Belimed GmbH`. Different legal entities in different countries. A
   merge verdict here is a false merge, and the cohort's framing invites it.
3. **Transliteration pairs, same country** — the Bulgarian hospital appearing
   as both `Universitetska mnogoprofilna bolnitsa…` and
   `УНИВЕРСИТЕТСКА МНОГОПРОФИЛНА…`. Cross-SCRIPT, not cross-border, and it
   rides in because a third member of the component carries another country.
4. **Same-country duplicates inside a "cross-border" component** —
   `CZ: Merck Life Science spol. s r.o.` x3 plus one SK sibling; two identical
   `DE: Vergabekammer Rheinland-Pfalz…` rows.

A single rubric cannot serve all four, and class 2 is the one that costs
something to get wrong. **The cohort needs splitting before a pilot**, not a
verdict vocabulary.

## A hypothesis I raised and then falsified

The cross-border country pairs include `BG-VU` (Vanuatu) 20, `BG-VA` (Vatican)
15, `BG-VE` (Venezuela) 13, `BG-VN` (Vietnam) 10, and the sample holds
`BF: БИС ООД` — a Cyrillic-named company under Burkina Faso. I took that for a
systematic country-code parsing bug (Cyrillic city prefixes read as ISO codes)
and checked it.

**It does not hold.** Those country codes carry real populations
(BF 120, VN 117, VU 112, VA 91, VE 61, GY 33) and the names under them are
genuine EU external-action entities: `Service européen pour l'action
extérieure au Burkina Faso`, `Bureau de la coopération suisse au Burkina
Faso`, `Helvetas Swiss Intercooperation`, local NGOs and consortia. EU
procurement covers development-aid contracts in third countries, so these
codes are legitimate.

What IS visible is a small misfiled residue under BF — `SKAMEX Spółka z
ograniczoną odpowiedzialnością` (Polish), `Veidekke Industri` (Norwegian),
`„OLI-NAT" Robert Zajkowski` (Polish). A handful, not a systematic fault.
Recorded here so the next reader does not re-raise the same alarm.

## Next unit, if this line is picked up

Split the 939 before building anything: separate same-country-pair components
(classes 3 and 4) from genuinely cross-border ones, and within those,
separate identical-name from near-name. Class 2 (corporate siblings) is
distinguishable only by judgement, which is what makes it the pilot's real
subject — and what makes a rubric written for class 1 dangerous.
