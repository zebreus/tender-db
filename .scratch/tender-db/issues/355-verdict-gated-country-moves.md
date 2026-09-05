# 355 — Verdict-gated country moves: the execution path `wrong-country` verdicts never had

Status: WET RUN DONE 2026-09-05 07:3x UTC (job 692: **153 rows moved, 0 no-ops, 135 land on a
standing duplicate identity**) after the classifier had refused the enqueue three times and
Lennart cleared it ("do not ask me for permissions again"). R2 dry (job 693: plan 102 groups; the other collisions sit in the consortium/legal-form
classes R2 refuses on purpose) and wet (job 699: **102 groups merged, 104 org rows removed,
797 mentions, 8,607 parties, 152 bid-parties, 505 winners repointed, 280 tenders touched**)
done; `project` (job 700) behind it. Audit read on `/v1/sql` after the fold: the moved rows
carry the corrected code (6861 SE, 9211279 IN, 13118097 FI, 13642179 MD); two of the four
pairs read are folded (their duplicates gone), two stand as same-triple duplicates R2's
guards refused (Indo UK Healthcare ×2 under IN, Turboenergy Power ×2 under MD) — the
duplicate-identity census's business now, not a country question. Built + deployed `75db3bf`; campaign: 487 cases reviewed and
challenged, 315 verdicts recorded under `xb-country-2026-09-05`, dry plan (job 691) exact
against the 153 recorded highs. Follow-up landed with issue 356: `from_country` now accepts
any published code as the pre-image (org 16789727 under `1A` is recordable). See "The
campaign" below.
Kind: capability (organization layer — data quality)
Relates to: 311 (the review loop), 314 (the same-name cross-border cohort whose verdicts are
inert), 326 (the same move, made by a predicate), 317 Unit A (the verdict/apply template)

## The gap

Issue 314's campaign recorded 124 verdicts on the same-name cross-border cohort — 62 of the
100 slice-1 verdicts say `wrong-country` — and its own status line says why they change
nothing: "Neither `merge` nor `wrong-country` has an execution path, so nothing applies from
this campaign yet." `org_case_reviews.handling` carries *which* row is wrong and *which*
country the evidence names as free text nothing can query. Slices 2–6 (465 → 363 cases after
the 325/326/328 repairs re-cut the cohort to 487) were held for that reason as much as for the
packet defect: at two agents a case, recording more inert verdicts is the wrong spend.

Issue 326's residue is the same shape from the other side: 36 clusters still multi-country
"because their stranger codes carry no evidence of a slip" and 153 `nobody-asked` — exactly
the cases a predicate abstains on and a per-case reader can decide, with no path to act on.

## Built

**`org_country_verdicts`** — one decision per organization row: `action` (`move`|`keep`),
`from_country` (what the reviewer SAW — the pre-image the apply pass re-checks; NULL is a real
pre-image for a country-less row), `to_country`, rationale, confidence, `applied_at` /
`applied_action` (the pre-image, so a move is reversible from the row alone) / `job_id`.
PK `(org_id, cohort)`. Re-recording replaces the verdict and never touches an applied stamp
(the 311 panel's catch, carried over from `record_rehoming`). A `move` without a destination,
to the code it already carries, or with a non-two-letter code is refused at record time.

**`POST /admin/country-verdicts`** `{cohort, verdicts:[{org_id, action, from_country?,
to_country?, rationale, confidence}]}` — validation mirrors `/admin/rehoming`.

**`apply-country-verdicts`** (`{"dry_run":false}` for wet; dry is the default). Eligible =
`move` × `high` × has `to_country`. The move is issue 326's statement for statement:
`UPDATE organizations SET country`, `append_change(organization, changed)`, `publish_cursor`.
The duplicate identity a move lands on (`collides_with`) is COUNTED and LEFT for the R2 arm —
a moved row lands on the triple it should have had all along. Guards: a row that no longer
stands under `from_country` (merged or moved since the review), a row already under
`to_country`, and a missing row are stamped no-ops with their reason, so they leave the pending
set and never retry. Dry stores the concrete plan as report `country-verdict-plan`
(org, from, to, kind, identifier, name, mentions, collides_with); wet reads it and executes
exactly those tuples or ABORTS. Arm body is `Box::pin`ned (the run_spec stack note).

Tests (`crates/store/tests/country_verdicts.rs`): the dry plan lists exactly the executable
moves and writes nothing; a wet run executes the reviewed plan, stamps pre-images and no-op
reasons, and a second run finds only the parked medium/keep; a stale plan refuses and writes
nothing; re-recording replaces the verdict but never the applied stamp (and another cohort's
verdict on the same row is a separate record); the record-time refusals.

## The campaign this unlocks (next unit)

1. Convert slice 1's 62 `wrong-country` case verdicts: one agent per case reads the recorded
   `handling`/`rationale` against the corrected packet's members and emits the structured
   `(org, from, to)` — or `needs-review` when the text does not name the row. Cheaper than
   re-review, and the packet's `probed && !agrees` / sibling-`agrees` evidence is the check.
2. Review the remaining ~363 same-name cross-border cases (v2 rubric, reviewer + challenger,
   verdict schema extended with `country_moves: [{org, from, to}]`) and 326's 36 + 153
   residue clusters — the cohorts a predicate abstained on.
3. Record → `apply-country-verdicts` dry → read the plan → wet → R2 dry/wet to fold the
   collisions → `project`.

## The campaign (2026-09-05, 38 agents, ~3.4M tokens, 2h40m at two concurrent on this 4-CPU box)

Shape: 17 stratified 35-case batches (identical/different identifier × none/one/several
`country_agrees`) on the fast model — one reviewer per batch with rubric v3
(`355-rubric.md`: v2's corrections plus the corrected packet's `probed`/`agrees`/`anchors`
fields, four evidence classes, HIGH bar), one adversarial challenger per batch (refute every
move literally against the fields); four blind second-reader batches (55 cases, every 9th) on
the session model. A first launch with the session model on 20-case batches was stopped after
40 minutes: ~30 min per reviewer at two concurrent would have taken a day.

Results: 487/487 verdicts. By stratum — identical/one-agrees 112 wrong-country of 125;
identical/none-agree 138 of 156; different/one-agrees 9 of 85 (57 distinct-entities);
different/none-agree 50 of 109 (31 same-entity-two-registrations); different/several-agree
0 of 12. 316 moves (225 high / 89 medium / 2 low; arithmetic 138, national-format 67,
name-language 73, weight 38). Challenger: agreed on 452 cases, disputed 34 moves, flagged 2
missed moves. Blind sample: 41/55 same verdict, 48/55 same HIGH move set — every
disagreement read: three are the reviewer over-claiming (all parked below), four are
the second reader finding a move the reviewer left as distinct/undecided (not applied;
a second-pass class).

**The deterministic floor** (the deny direction, in code over the reviewers' output — 53
highs parked to medium): 36 `identical-identifier-weight` moves with no anchor anywhere in
the case; 6 arithmetic moves between CZ/SK/SI, whose IČO/davčna share one mod-11 rule while the
vocabulary carries no SK scheme (a CZ anchor on an unprobed SK row is a coincidence-proof hit,
not a discriminator — 314's own note, re-found by the challengers; two of those would have moved
a Slovak row with the standing, one named "Basco SK"); 11 moves inside a shared REGISTER
(Åland↔Finland, Réunion↔France: one register serves both codes, so a validating number cannot
separate the tags — a policy question, not a contamination). Plus 34 challenger-disputed
highs. Apply set: **153 HIGH** (arithmetic 105, national-format 58): SE→FI 14, NO→DK 14,
NO→SE 11, AD→CZ 8, SE→DK 7 … The six heaviest (9–23 mentions: a 13-digit Moldovan IDNO under
RO, an identical IT partita IVA beside a 391-mention agreeing IT row, a complete Indian CIN
under GB, an 8-digit FI y-tunnus under SE, a 14-digit FR SIRET under ES) were read by hand
and hold. Full record: `355-xb-country-verdicts.json` (verdicts, moves with the recorded
confidence, challenger notes, blind-sample verdicts).

Two things the campaign found that the rules had not: (1) issue 325's parse-artefact class
decided case by case — HRB register citations under NL/GE, Irish CHY charity numbers under
CH, `Registergericht München HRB…` as an identifier under SE; (2) one recorded row could not
be posted at all: org 16789727 stands under country `1A`, and `from_country` must be a
two-letter code — a junk code is exactly what a contamination looks like, so the record-time
check should accept any published code as the pre-image (small follow-up; the row is a
medium anyway).

## Calibration against slice 1 (read through issue 356's endpoint, 2026-09-05)

The 124 `xb-same-name-2026-08-31` case verdicts (session model, rubric v2, 2026-08-31) cover
99 components still in the re-cut cohort. Against this campaign's verdicts on the same 99:
**88 in the same class** (52 wrong-country, 25 distinct-entities, 10 same-entity-two-
registrations, 1 needs-more-evidence). The 11 disagreements are all inside the soft classes
(same-entity ↔ needs-more-evidence 4, distinct ↔ needs-more-evidence 2, prior merge → new
wrong-country 2 with NO high move, prior wrong-country → new same-entity/needs-more 2, one
distinct ↔ same-entity). None touches the applied set. Two independent reviewer populations,
two rubric versions, one contamination finding.

## Not in scope

A `merge` verdict's execution path (a review-gated merge arm writes entity references and
needs its own dry-first plan and panel round — 314's constraint stands). Moving the
identifier or the name: this rewrites `country` only.
