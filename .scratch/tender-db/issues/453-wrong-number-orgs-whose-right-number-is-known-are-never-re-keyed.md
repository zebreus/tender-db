# 453 — 182 wrong-number organizations have a reviewer-found right number that nothing acts on

Status: ready-for-agent — filed 2026-09-30 from issue 452. Next: for each of the 182, look up whether the right number
already has its own org (a bounded id/identifier lookup), then size the three shapes below before building anything.
Kind: data quality (identifiers)
Relates to: 452 (the verdicts), 448 (the altid arm), 362 (merge verdicts)

## What is there

Issue 452 posted 685 identifier verdicts (cohort `452-census-2026-09-30`). 371 are `wrong`, and for **182** of those the
reviewer found the organization's real company number, recorded as `correct_identifier`. 83 of the 102 `related`
verdicts carry one too. Nothing reads it. By design (452): re-keying an org is merge-shaped, because the right number
may already have its own org.

Today such an org is withheld from identifier matching and flagged `register_mismatch`. It stands alone under a wrong
number: its tenders do not join its real registration, and a reader who follows the right number does not find them.

## The three shapes

For each `wrong` verdict with a `correct_identifier` C:
1. **C has a standing org R**, the same entity: merge the wrong-number org into R. This is a merge verdict under a new
   scheme (for example `GB:rekey`, key `<wrong>~<C>`), reviewed members, the 362 execution path. R keeps its own
   identity.
2. **C has no org:** re-key the wrong-number org to C in place. The identity change is a move (the 355 shape: pre-image
   in the verdict row, change event, reversible). Afterwards the 452 verdict is inert by construction, because the org
   no longer carries the number.
3. **C has an org that is NOT the same entity** (the reviewer's C is itself wrong): leave it and record why.

## Verify

    /root/aj.sh "/admin/case-reviews?table=identifier&cohort=452-census-2026-09-30&limit=5000" | python3 -c "import json,sys; d=json.load(sys.stdin); print(sum(1 for r in d['rows'] if r['verdict']=='wrong' and r['correct_identifier']))"

- **open**: `182` (every one still carries its wrong number with a known right one beside it).
- **done**: the count of verdicts whose org still carries the wrong number while a right one is known reaches 0, or
  each remaining one has a recorded shape-3 reason.
