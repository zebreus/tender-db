# 355 — Verdict-gated country moves: the execution path `wrong-country` verdicts never had

Status: BUILT 2026-09-05 (gate running) — `org_country_verdicts`, `POST /admin/country-verdicts`,
`apply-country-verdicts` (dry default; wet executes the reviewed (org, from, to) tuples or
refuses), five store tests. Deploys at the next idle window; then the campaign below feeds it.
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

## Not in scope

A `merge` verdict's execution path (a review-gated merge arm writes entity references and
needs its own dry-first plan and panel round — 314's constraint stands). Moving the
identifier or the name: this rewrites `country` only.
