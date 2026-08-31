# 314 — Stage 4's 1.5M candidate edges have no consumer

Status: RUBRIC VALIDATED 2026-08-31 by a controlled A/B — the de-biased rubric
drops disputes to ZERO and the contamination finding survives it. Ready to
batch the 589
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

## THE SPLIT, measured on prod (2026-08-31, job 533, rev 4c16655)

    cohort              939
      same-name         589   62.7%
      diff-name         129   13.7%
      with-intra        221   23.5%
    partition check     939 = 939   (every component in exactly one class)

Classification: `same-name` = every member's country distinct AND all names
normalize alike; `diff-name` = countries distinct, names differ;
`with-intra` = two members share a country. Case and punctuation alone do not
split a component — the corpus pairs `SARSTEDT spol. s r.o.` with `Sarstedt
spol. s r.o.`, and counting that as diff-name would drop a plain duplicate into
the dangerous class.

### What each class actually holds, from the live sample

**same-name (589)** — the pilot cohort. Two sub-shapes, both pointing the same
way (unify):

    DK/LT/NO: Mercell Holding ASA          one entity, three country codes
    CZ/SK:    SARSTEDT / Sarstedt          one entity, case variance
    ES/NL:    Howden Iberia S.A.U          one entity, punctuation variance
    CH/EE:    Gemeinde Glattfelden…        one entity, one WRONG code

**diff-name (129)** — do not write the pilot's rubric here first:

    AT/DE:    Steelco Belimed GmbH | Belimed GmbH        siblings, must NOT merge
    EE/LV:    Lanmer Group OÜ | Lanmer OÜ                unclear
    FI/RO:    SERVICIUL JURIDIC… | RETELE ELECTRICE…     a DEPARTMENT of the other
    BG/GY:    СТОЛИЧНО ПРЕДПРИЯТИЕ… | Столична община…   same entity, different granularity

Note this class is not purely siblings — it also holds same-entity pairs
recorded at different granularity (a department beside its parent). So
"distinct entities" is not its default answer either; it is the class that
genuinely needs judgement, which is the argument for piloting it SECOND, on a
rubric informed by the first.

**with-intra (221)** — settle the intra-country duplicate first:

    CZ x3 + SK:  Merck Life Science spol. s r.o.
    DE x3:       Vergabekammer Rheinland-Pfalz…
    BG x2 + VE:  the Cyrillic/Latin transliteration pair
    AT x2 + DE:  PROSE GmbH

## Next unit

Pilot on a sample of the **589 same-name** components with a merge / not-merge
/ unsure rubric. Verdicts land as records only — the 311 apply job's safe
subset is an identifier strip, and a merge verdict has no execution path yet
(this issue's item 4 still stands). The 129 diff-name components want their own
rubric afterwards, and the 221 with-intra ones belong to whatever settles
same-country duplicates.


## THE PACKET, and what it changed (2026-08-31, job 534, rev 291a318)

`xb-packet` carries all 589 same-name components in **5 seconds**. The count
matching the census's 589 exactly is the check that mattered: the packet
restates the same-name definition independently, so equal counts mean the two
did not drift.

    component sizes:  2 → 578,  3 → 11
    mention spread:   lopsided (>=80% on one row) 196 | even 393
    members carrying name variants: 589 of 589

**That last number is a correction to my own commit message.** I wrote that a
satellite carrying the other member's spelling is "different evidence from a
bare normalized-key match". It is not distinguishing at all — EVERY case has
variants on some member, because the resolver writes a variant on every mention
capture. Variants are context for a reviewer, not a signal.

### The discriminator is the identifier, and it was not foregrounded

Splitting the 589 by how the members' identifiers relate:

    identical-identifier       336   57.0%
    different-identifiers      200   34.0%
    platform-guid               31    5.3%
    same-digits-diff-format     22    3.7%

    identical    NO:980921565(29524) | DK:980921565(2) | LT:980921565(2)   Mercell
                 ES:A82473349(27)    | NL:A82473349(1)                     Howden Iberia
    different    CZ:43000916(87)     | SK:31359825(2)                      Sarstedt CZ vs SK
                 CZ:27599876(65)     | SK:35975075(6)                      Explorea CZ vs SK
    guid         CH:49A0CF…(1)       | EE:EEE9F4…(1)                       platform keys
    same-digits  AX:FONR02295252(1)  | FI:02295252(2)                      register prefix
                 FI:FI07545185(19)   | SE:07545185(1)                      VAT prefix

**336 cases carry the SAME identifier value across countries.** One entity, one
registration, several country codes — Mercell's Norwegian id filed under DK and
LT on two notices each while the NO row holds 29,524. Those need no per-case AI
judgement; they need the R2/R3 arms' existing same-identifier logic, which
currently declines them only because the COUNTRIES differ.

**200 carry different identifiers** — `SARSTEDT spol. s r.o.` on CZ 43000916
beside `Sarstedt spol. s r.o.` on SK 31359825. Two national registrations, two
legal entities, same name. These must not merge, and they are where judgement
is actually needed.

**22 are the same digits in different formats** — `FI:FI07545185` vs
`SE:07545185`, `AX:FONR02295252` vs `FI:02295252`. A register or VAT prefix,
which is Stage 2's canonicalization problem reappearing across a border.

## What this means for the campaign

The AI review campaign this issue proposed is **the wrong first instrument for
57% of its own cohort**. The identical-identifier 336 are a rule-shaped class:
same normalized name AND byte-identical identifier, differing only in country.
Reviewing them one case at a time would spend ~1.8M tokens re-deriving what one
predicate already says.

Revised order:

1. **336 identical-identifier**: extend the R2/R3 merge arms with a
   cross-border same-identifier arm, gated and dry-first like every arm before
   it. No review campaign.
2. **22 same-digits**: Stage 2's canonicalization, applied across countries.
   Also rule-shaped.
3. **200 different-identifiers**: THIS is the AI review cohort — 200 cases, not
   589, and every one genuinely needs judgement about whether two national
   registrations are one entity.
4. **31 platform-guid**: a different question (both sides are platform record
   keys, one country is contaminated), and it belongs with issue 312's
   GUID-class thinking rather than here.

Lennart's steer in issue 311 is that rules are the deny-direction floor, not
the detector. That holds — but it does not mean every cohort a rule cannot
FULLY settle should go to review whole. Here a rule settles 61% of it, and the
remaining 39% is a better campaign for being smaller.


## THE CHECKSUM READ (2026-08-31, rev b4a18a2) — and a correction

Last entry I wrote that the 336 identical-identifier cases are "rule-shaped"
and need "no review campaign". **That was too broad.** Adding the checksum
evidence to the packet splits those 336 three ways:

    one country agrees, others CONTRADICTED   129   38%
    no arithmetic evidence                    140   42%
    anchors, but NO country agrees             67   20%

**129 — the country-contamination class, and the only rule-shaped one.**

    NO:980921565 (NO:orgnr, agrees) | DK:980921565 (NO:orgnr, X) | LT:… (X)
    NO:5569584120 (SE:orgnr, X)     | SE:5569584120 (SE:orgnr, ok)

One member's country matches the value's arithmetic and the others do not. The
repair is a COUNTRY CORRECTION on the contradicted rows, after which the
existing same-country same-identifier logic merges them through a tested path.
No new merge arm, and the correction preserves the fact that a country code was
wrong instead of burying it in a merge.

**140 — no arithmetic evidence.** `ES:A82473349 | NL:A82473349` is a Spanish
CIF; the probe declines letter-bearing values by design (they are the
register-prefixed form, out of its scope). Nothing to conclude either way.

**67 — the cautionary bucket, and the reason the rule needs its guard.**

    LT:300112408 | LV:300112408   both "anchor" PT:nif
    IT:0072080930 | SK:0072080930 both "anchor" SE:orgnr
    SK:37006771  | SI:37006771    both "anchor" CZ:ico

Neither row's country appears in the anchors at all. These are COINCIDENTAL
checksum passes — a Lithuanian number that happens to satisfy the Portuguese
NIF arithmetic proves nothing about Portugal or about either row. So "the
country disagrees with the anchor" is only evidence when EXACTLY ONE member
agrees; when none does, the anchor is noise and must not be read as
contamination on both sides.

### Revised, again

    129  country correction, rule-shaped, reuses tested merge machinery
    207  (140 + 67) identical identifier, no usable arithmetic evidence —
         these need the mention spread, the notices, or review
    200  different identifiers — the genuine AI review cohort
     31  platform GUIDs — issue 312's class
     22  same digits behind a register/VAT prefix — Stage 2 canonicalization

The honest shape of this line of work keeps being that each measurement shrinks
the part a rule can settle and sharpens the part that cannot. 589 → 336 → 129.


## THE RULE-HUNT, AND WHY I AM STOPPING IT (2026-08-31)

I proposed a country-correction rule for the 129 and then tested whether the
mention weight backs the checksum-agreeing row. It does not:

    agreeing row holds >=90% of mentions                      28   22%
    agreeing row holds 50-90%                                 81   63%
    agreeing row holds <50% — the CONTRADICTED side is heavier 20   16%

The 63% middle is mostly 2-vs-1 and 3-vs-1: noise, not evidence. And the
counter-examples are instructive rather than freak:

    'emmaus åland rf'    agrees FI=1  vs contradicted SE=2
    'kristers åkeri'     agrees FI=1  vs contradicted SE=2
    'in via'             agrees FI=1  vs contradicted SE=2

Åland — an autonomous, Swedish-speaking region **of Finland**. The arithmetic
says FI, the notices say SE, and both are defensible readings of a real
organisation. No predicate over identifier-vs-country settles that; knowing
what Åland *is* settles it.

### The deeper problem the numbers only illustrate

"Identifier and country disagree" never says WHICH field is wrong. A Norwegian
subsidiary of a Swedish parent, correctly filed under NO but carrying its
parent's Swedish org number, looks identical to a Swedish company mis-filed
under NO. The first wants its identifier stripped; the second wants its country
corrected. The evidence in the packet cannot tell them apart, and a rule that
picks one will be confidently wrong on the other.

### The narrowing, in one line

    589 same-name  →  336 identical-identifier  →  129 one-country-agrees  →  28 with decisive mention weight

Each measurement shrank the part a rule could settle by roughly half, and the
28 survivors are a hand-review, not machinery. **I am stopping the rule-hunt
here.** Three firings of narrowing produced a candidate class 4.8% the size of
where it started, and the residue at every step was "this needs someone to know
what the organisation actually is".

## This is Lennart's steer, arrived at the long way

Issue 311 opens with it: *"nearly all errors can not be detected by simple
rules... they need manual ai agent review for each individual case and how to
handle it."* I spent three rounds looking for the rule-shaped subset anyway,
which was worth doing — the measurements are real, the packet is built, and the
cohort is now well-characterised rather than assumed. But the answer the
measurements keep returning is the one the issue stated at the start.

**Next unit: run the review campaign on the 589**, with the packet's evidence
(rows, identifiers, checksum anchors, mention spread, variants, notices) and a
rubric whose verdicts are merge / distinct-entities / wrong-country /
wrong-identifier / needs-more-evidence. Verdicts as records only — a merge
still has no execution path (this issue's item 4, still open). The 28
decisive-weight cases are worth reading first as gold exemplars, since their
answer is already legible from the evidence.

## THE PILOT (2026-08-31, 24 cases, 48 agents: one reviewer + one adversarial challenger each)

    wrong-country       19
    distinct-entities    3
    merge                1
    needs-more-evidence  1
    disputed by the challenger: 2
    confidence: 19 high, 5 medium

    by stratum
      same-id-decisive   7 wrong-country
      same-id-ambiguous  7 wrong-country
      diff-id            3 wrong-country, 3 distinct-entities, 1 merge
      guid               2 wrong-country, 1 needs-more-evidence

**Country contamination is the dominant class, and it is far wider than the
checksum evidence suggested.** All 7 `same-id-ambiguous` cases — the ones where
the checksum said nothing usable — came back as contamination once a reviewer
could read the NAME. `Gared s.r.o.` under **AD** (Andorra), `Festing, s.r.o.`
under **HK**, a Czech school canteen under **AD**, Bulgarian EOOD companies
under **VN**, **VG**, **BW**, **BR**, a Lithuanian UAB under **MT**. The legal
form in the name settles what no checksum could: `s.r.o.` is not an Andorran
company form, `EOOD` is Bulgarian, `UAB` is Lithuanian.

The 3 `distinct-entities` are exactly the shape predicted: different national
register numbers on each side (Ceemed SK 45730661 vs CZ 24671819; Mara SI vs
HR; Ergonomik EE vs FI). The rubric separated them cleanly.

## The pilot caught a bias I put in its own rubric

Between a 2-case smoke test and the full run I edited the rubric, adding:
*"Use this rather than 'merge' when the rows are one entity AND you can say
which country codes are the mistake — it is the more specific finding."*

That nudge skewed the distribution, and the challenger caught it twice:

- **Platform 24 Healthcare AB** (SE orgnr + DE VAT). Under the PRE-patch rubric
  the smoke test returned `merge`. Under the patched rubric the same case
  returned `wrong-country` — and the challenger disputed it: `DE349977421` is a
  German-issued VAT number, so tagging that row DE names a real jurisdiction.
  `merge` was right and the patch pushed the reviewer past the evidence.
- **BIEGE Neubaustrecke Dresden-Prag** (identical platform GUID, DE and AT).
  One record ingested twice; the `c/o ILF Consulting Engineers Austria GmbH`
  clause reads both ways, so the DE-vs-AT direction is not determinable.
  `wrong-country` again over-reached.

**Same case, two rubrics, two verdicts, and the pre-patch answer was the right
one.** The lesson is not "the rubric was bad" — the multi-subject fix in the
same edit was necessary and correct (case 929 needed to name two contaminated
rows). It is that "prefer the more specific verdict" is an instruction to
over-claim, and an adversarial second pass is what makes that visible.

### Rubric changes before any batch

1. **Drop the prefer-wrong-country sentence.** Keep the verdict; remove the
   nudge. A reviewer should reach for `wrong-country` when it fits, not
   because it scores higher.
2. **Keep the multi-subject `subject_orgs`.** It was the other half of the same
   edit and it was right.
3. **Add to the evidence key: a foreign VAT number names a real jurisdiction.**
   A row tagged with the country that ISSUED its VAT id is not contaminated.
4. **Keep the challenger.** It disputed 2 of 24 (8%) and both disputes were
   correct. That is the pass that turns a plausible campaign into a checkable
   one.

## Next

Re-run the pilot on a fresh stratified sample with the corrected rubric, confirm
the `wrong-country` share drops to something the evidence supports, then batch
the 589. Verdicts still record-only: a merge has no execution path, and now
neither does a country correction.

## THE CONTROLLED A/B (2026-08-31) — same 24 cases, one variable changed

I re-ran the identical 24 cases with the nudge removed and the foreign-VAT note
added. Nothing else changed: same inputs, same challenger, same schema.

    v1 (with "prefer wrong-country")   19 wrong-country · 3 distinct · 1 merge · 1 unsure   2 DISPUTED
    v2 (nudge removed)                 16 wrong-country · 3 distinct · 4 merge · 1 unsure   0 DISPUTED

**Three of 24 flipped (12.5%), and they are exactly the right three:**

    22702360  Platform 24 Healthcare AB   SE orgnr + DE VAT       wrong-country → merge
    22241904  BIEGE Dresden-Prag          identical platform GUID  wrong-country → merge
    19840642  GIST Research Ltd           UK vat + GB vat          wrong-country → merge

The first two are the cases v1's challenger disputed. The third is one the v1
challenger did NOT catch, and the de-biased reviewer got right on its own — so
removing the nudge did more than undo the two known errors.

**Disputes went 2 → 0.** With the nudge gone, the adversarial pass finds nothing
to dispute across all 24. Reviewer and challenger now agree completely, which is
what a calibrated rubric looks like.

**And the headline finding survives the change.** All 21 unchanged cases stayed
put, including every same-identifier case as `wrong-country`. So the country
contamination is a property of the data, not an artefact of how I asked — which
is the thing that most needed checking before batching.

### The lesson, stated for whoever runs the next campaign

"Prefer the more specific verdict" reads like a quality instruction and behaves
like a pressure to over-claim. It cost 3 of 24 verdicts, and every one of them
pushed a defensible `merge` into an unsupported `wrong-country`. The fix was one
sentence removed. The reason it was findable at all is the adversarial second
pass plus an accidental 2-case control run — neither of which was in the plan.

## Next

Batch the 589 with the v2 rubric. Verdicts record-only via
`POST /admin/case-reviews` under cohort `xb-same-name-2026-08-31`, keyed by
component root. Keep the challenger on every case — at 0 disputes it is now
cheap insurance rather than a filter, and the run where it stops being cheap is
the run that needs it. Neither `merge` nor `wrong-country` has an execution
path, so nothing applies from this campaign yet.

## Slice 1 of 6: 100 cases reviewed, and the packet's own evidence was defective

2026-08-31. 100 cases, 200 agents (reviewer + challenger), 0 errors. Verdicts:

| verdict | n |
| ------- | - |
| `wrong-country` | 62 |
| `distinct-entities` | 24 |
| `merge` | 13 |
| `needs-more-evidence` | 1 |

All 100 recorded on prod: `{"cohort":"xb-same-name-2026-08-31","recorded":100}`,
after the 24 pilot verdicts under the same cohort. The headline holds: country
contamination dominates the same-name cross-border cohort.

**But 6 cases came back disputed, and all 6 share one shape** — the challenger
agreeing the members are one entity while refusing `wrong-country` because
nothing in the packet said WHICH country code was wrong. That is not a reviewer
problem. It is a defect in the evidence I built.

`country_agrees = false` meant three different things at once:

1. the arithmetic was run under the row's own country's scheme and refused it,
2. the row's country has **no scheme of this value's shape**, so nothing was
   ever asked,
3. the value anchors nowhere at all.

Only (1) is the contaminated-country signal. A challenger caught it precisely:

> no SK:ico scheme appears in the anchor vocabulary at all, so
> country_agrees=False on the SK row is likely a vocabulary gap

— and noted that CZ and SK IČO share the same mod-11 checksum, so a CZ anchor is
not even weak evidence against SK. Same for DK: `DK:cvr` is an 8-digit scheme, so
a 9-digit value on a DK row was never tested as DK.

**This is issue 318's failure again**: a zero that means two things, read as
though it meant one. 318 cost three rounds of the same mistake at three levels;
here it cost 6 of 100 verdicts. The pattern is now twice-confirmed and worth
stating as a rule: *any boolean whose false branch has more than one cause has to
carry the cause beside it, or a reader will pick the interesting one.*

### The fix

`XbMember` gains `country_probed`, and `xb_same_name_packet` takes a second
injected probe, `ingest::idgate::anchor_vocabulary`, reporting what the checksum
probe **asks** rather than what **passes**. A reviewer now separates:

* `agrees` — the arithmetic works where the row says it is;
* `probed && !agrees` — tested under the row's own register and refused,
  accepted under another's → the contaminated-country signal, and the repair is
  a country correction through an already-tested path;
* `!probed` — no scheme of the row's country has this shape. Silence with no
  content.

`checksum_anchors` and `anchor_vocabulary` now read **one table**
(`uniform_arm`) and share the 14-digit `ambiguous_pad` judgment, because a drift
between them would put a lie in a reviewer's evidence — a refusal-to-decide
reported as a question that was asked. Two parity tests guard it: everything that
can pass must be admitted as asked, and a declined shape must not be reported as
asked.

The store test that asserted this behaviour has been rewritten, because **it
encoded the same over-claim**: it set up a 9-digit NO-passing value on a DK row
and asserted "DK is the contaminated side". DK's scheme is 8-digit; DK was never
tested. The test now asserts all three states on one value — NO agrees, PT is
probed and refused, DK is unprobed — and its fake vocabulary keeps production's
decisive property, that the arms are disjoint by length.

### Slices 2–6 are held

465 cases remain. They are NOT running until the corrected packet is deployed
and rebuilt: running them now would bake the same over-claiming into 465 more
verdicts, at 2 agents each. The 100 recorded verdicts stand — 94 of them were
never in question, and the 6 disputed ones are recorded as disputed, which is the
record behaving correctly.

### Two exclusions for the remaining slices

40 of the 589 cases (6.8%) contain a member whose country is a **parse artefact**,
now filed as issue 325: the VAT-prefix sniffer mints a country from any word
whose first two letters spell one (`CHARITYNO…` → CH on a UK charity,
`BERICHTSEINHEITID…` → BE on German public bodies, `FIRMENBUCHNUMMER…` → FI on
an Austrian GmbH, 32-char hex GUIDs → EE/BE/DE). 4,206 rows corpus-wide. Those
40 cases should be excluded from slices 2–6 rather than hand-reviewed — a
predicate decides them.

Credit where it belongs: three of slice 1's 100 rationales independently named
that mechanism from a single case each. The aggregate measurements had not seen
it in four passes over this cohort.

### The corrected packet, measured on prod — the field was carrying 0.8% of what it claimed

Deployed `b1797a2`, rebuilt the packet (job 536), and read the evidence split
across all 589 cases / 1,189 members:

| state | members | share |
| ----- | ------- | ----- |
| never asked, anchors nowhere | 590 | 49.6% |
| never asked, but anchors elsewhere | 348 | 29.3% |
| agrees | 243 | 20.4% |
| **PROBED and refused** — the contamination signal | **8** | **0.7%** |

**946 members read as a single `country_agrees = false` under the old packet.
Eight of them — 0.8% — were the thing the field was documented to mean.** The
other 938 were a vocabulary gap wearing the same face. My estimate before
measuring was that the conflation had cost 6 disputed verdicts; the real ratio is
an order of magnitude worse than that, and the disputes were only where the
challengers happened to look.

### But the verdicts did not rest on it — and that is the more interesting finding

Auditing slice 1's 62 `wrong-country` verdicts against the corrected evidence:

| what the verdict rested on | n |
| -------------------------- | - |
| only never-asked silence + an anchor elsewhere | 31 |
| no anchor evidence at all | 28 |
| **arithmetic that was actually asked** | **3** |

Three of sixty-two. The reviewers were not using the checksum evidence; they were
reading things the packet reports but does not reason about:

* **an identical identifier across a one-letter country pair** — SK/SG, CZ/CR,
  BG/BF. Now filed as issue 326: 76 of 589 cases (12.9%), and **18 of 18** of
  those that reached slice 1 came back `wrong-country`.
* **a recognizable national format** — `A82473349` is a Spanish CIF, so the NL
  row is the wrong one. No checksum needed.
* **a positive anchor on the sibling row** — `5569584120` validates as a Swedish
  orgnr, so the NO row is wrong. The evidence is the *other* member's `agrees`,
  not this member's `false`.
* **an issue-325 parse artefact** — 40 of 589 cases (6.8%).

So the honest verdict on the campaign is mixed and both halves matter: the
packet's flagship evidence field was nearly useless, the reviewers routed around
it without being told to, and their conclusions stand on other grounds. That is
the second time this campaign that the agents' *rationales* — not their verdicts
— were the valuable output.

### Re-scoping slices 2–6

Issue 325's 40 cases and issue 326's 76 do not overlap at all. Together
**116 of 589 — 19.7% of the cohort — is decidable by predicate**, and should be
excluded from the remaining slices rather than reviewed at two agents apiece. The
campaign's job is the residue that genuinely needs judgement; spending it on
pattern-matching a census can do is the thing to stop.

Slices 2–6 stay held until 325 and 326 have censuses, so the exclusion list can
be computed instead of estimated.
