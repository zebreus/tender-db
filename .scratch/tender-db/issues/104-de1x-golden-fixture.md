# 104 — a DE-1.x golden fixture: close the epoch-discipline hole and the fold-order gap at once

Status: DONE 2026-08-20 (owner) — the golden corpus now carries the DÖE↔TED cross-source pair
(BT-04 `1af86e3c…`, one Tender, two versions, structural assertions beside the byte digest) and both
DE-1.x fixtures, and the snapshot's first line is PROJECTION_EPOCH so a regeneration diff puts the
bump in front of whoever makes it. Falsified in both directions — including one instructive miss: the
first probe (dropping the DE1 OPT-320 alias) left the golden GREEN, because the DE-1.x results graph
carries a redundant second path to the bid (LotResult→SettledContract→LotTender via OPT-315+BT-3202)
and `bind` prefers the contract leg; a probe on the un-twinned BT-720 value alias turned it red. The
guard fires on OUTPUT changes, which is what it pins — and the redundancy it exposed is now recorded
below. Was: open, required follow-up.
Kind: test coverage / process guard
Blocked by: —
Relates to: 99 (the discipline this guards), 98 (the change it would have caught), 94 (the allowlist
gate that covers the other half), 85

## Two holes, one fixture

**1. The epoch bump is human discipline, and its only guard has a gap.**

Issue 99's `PROJECTION_EPOCH` must be bumped by hand on any projection-logic change. The mitigation on
record is that `project_golden` pins fold output, so a logic change turns it red and forces a
deliberate regeneration — putting the bump on the road the change already travels.

That guard does not cover the case that actually happened. **`project_golden`'s corpus contains no
eForms-DE 1.x notice** (`grep -c "eforms-de-1" project_golden.rs` → 0), and `normalise_de1` is
profile-gated, so a DE-only change — issue 98 exactly — **cannot** turn golden red. The coupling would
not have prompted the very bump it exists to prompt.

The same is true of `project_equivalence` (its corpus is `eforms:eforms-sdk-1.13` and `ted-export-r209`
only). So "golden and equivalence stayed green" proves DE-path changes are safe only in the sense that
they are *invisible* to those suites.

**2. The DE path has no version-count or fold-order coverage.**

Multi-notice DE *grouping* is covered in both directions (`a_uuid_de1_folder_id_still_merges_the_procedure`,
`a_non_uuid_de1_folder_id_does_not_merge_notices`). Neither asserts version **count** or **order**, so
the `published_at` lever — which orders versions within a Tender via `plan_notice_fold` — has no
behavioural test. Issue 94's alias allowlist gate *prevents* an alias from reaching that lever, which is
stronger than detection, but it does not cover a fold-order change arriving by any other route.

## The fixture

A DE-1.x notice **and its TED twin** sharing a BT-04 / folder uuid, in the golden corpus, with distinct
`published_at` so their fold order is meaningful. That single addition:

- makes any DE-1.x projection-logic change turn `project_golden` red, so the epoch bump is prompted on
  the path the change already travels;
- gives the DE path multi-notice **version-count and fold-order** coverage;
- exercises the cross-source merge that is the cohort's dominant real shape (216,450 of the 218,635
  merged onto TED twins), which no current fixture does.

## Acceptance

- `project_golden` fails if a DE-1.x mapping changes without the golden being regenerated.
- The fixture asserts one Tender, two versions, ordered by `published_at`, with the DE notice's facts
  and the TED twin's both present.
- Regenerating the golden surfaces `PROJECTION_EPOCH` (include it in the fixture) so the bump is not
  silently skipped.

## Note

Not blocking the issue-98/99 re-fold: the production `EXPECT_NO_REGROUPING` backstop is the full-scale,
real-chain check for that run. This is load-bearing for the **next** profile-specific change, when
nobody is watching the fold as closely.

## Done (2026-08-20) — and what the falsification taught

Corpus added to `project_golden`: the `doe-ted-pair` (its DÖE half recorded with source `doe`, which
required the golden's ingest helper to take a source at all — it had "ted" hardcoded, itself a symptom
of the blind spot this issue names) and both committed DE-1.x fixtures. The snapshot now begins with
`--- projection epoch ---` so a regeneration diff surfaces the bump on the road the change travels.
Structural assertions ride beside the byte digest: the pair folds to ONE Tender with TWO versions in
`published_at` order, so a cross-source-merge failure says so in words rather than as a byte diff.

One deviation from the spec as written: the pair's two notices carry the SAME `published_at` (both
sides published the same day, which is the normal real shape — the DÖE mirror and the TED gazette
publish in lockstep). The fold order between them is therefore decided by the source-rank tiebreak,
not the date — still deterministic, still pinned byte-for-byte in the digest, and the order assertion
is `<=` accordingly. A pair with genuinely distinct dates would need archive excavation for marginal
extra value; not done.

**The falsification found a redundancy worth knowing.** Probe 1 removed the
`DE1-NoticeResult-LotResult-LotTender-ID → OPT-320-LotResult` alias — the edge issue 100's whole
diagnosis walked through — and the golden stayed GREEN, with even the direct issue-100 test passing.
Not a broken guard: the DE-1.x CAN publishes the LotResult→bid edge TWICE, once directly (OPT-320)
and once through the contract (OPT-315 → SettledContract → BT-3202 → LotTender), and `bind` prefers
the contract leg when contracts resolve. Removing either lone edge changes no output, so nothing
output-pinning can catch it — and that is fine, because output is the thing being guarded. Probe 2
(the un-twinned `BT-720-Tender` value alias) turned the golden red on the first run. Consequence
worth stating: the winner chain issue 100 fixed is TWO-legged on real DE-1.x CANs, so it degrades
gracefully if either leg's mapping regresses — but a regression of BOTH legs at once would read as
"winners gone", and the golden now catches exactly that.
