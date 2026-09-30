# 453 — 182 wrong-number organizations have a reviewer-found right number that nothing acts on

Status: ready-for-agent — SIZED 2026-09-30 15:0x UTC: of the 182, **119** have a standing GB org carrying the right number (shape 1, merge into it) and **63** have none (shape 2, re-key in place). No existing arm executes either: case reviews only strip, merge verdicts need a shared key group. NEXT: the arm (design below), dry-first.
Was status: ready-for-agent — filed 2026-09-30 from issue 452. Next: for each of the 182, look up whether the right number
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

## 2026-09-30 15:0x UTC — sized

A bounded identifier seek per right number, both spellings (`C`, `GBCOHC`), GB only
(`453-rekey/shapes-2026-09-30.json`):

| shape | orgs |
|---|---|
| 1: an org already carries the right number | **119** |
| 2: no org carries it | **63** |

The shape-1 samples read as one entity on both sides:
- Rolls-Royce plc 01006142 → org 17148236 (01003142, Rolls-Royce Plc);
- Gasway 01458628 → 8879818;
- Fixatex 02791975 → 18010487;
- Firmus Energy 05369180 → 31547550 (GBCOH05369108);
- RSK Environment 05837803 → 10888303 (SC115530).

Shape 3 (the right number's org is someone else) is decided per pair by the name gate below.

**Why nothing existing does it.** `apply-case-reviews` executes one action only: stripping an identifier. `org_merge_verdicts`
are honoured only for a group that shares one canonical key, and a wrong number and its right number never share one.

**Design for the arm (next unit).**
- `rekey` as a rule of `match-org-identifiers`, planned from `org_identifier_verdicts` rows with `verdict = 'wrong'`
  and a `correct_identifier`, restricted to orgs that still carry the wrong triple.
- **Shape 1 is a merge.** The wrong-number org is the loser and the right-number org survives, through
  `repoint_org_references`. That is the R2/altid merge body: ledger rule `r-rekey`, change events. Gates: one owner of
  the right number (several is a family R2 declined), the name agreement the altid arm uses (`names_agree` over both
  orgs' heads and satellites), the legal-form veto, and the consortium veto. Dry run lists every pair, and the wet run
  holds against the listing's pairs, as the altid arm does.
- **Shape 2 is a move.** It rewrites the org's `identifier` to the right number's canonical spelling. Pre-image: the
  verdict row already keeps the wrong literal, and the move adds `applied_at`/`applied_action`. It fires an
  `organization changed` event. After the move the 452 verdict no longer matches the org, which is the intended end state.
- Tests: both shapes, the name-gate refusal (shape 3), the several-owners refusal, and parity.
