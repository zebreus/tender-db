# 194 — the VEAT form family is unclaimed: 2,380 notices held on VOLUNTARY_EX_ANTE_TRANSPARENCY_NOTICE, plus the defence-form and internal-ojs residues

Status: IN PROGRESS (2026-08-13) — the internal-ojs slice is DONE (6 _SUM aliases, all 7 members verified parsing, fixture 116870 committed); VEAT + defence-form rule mining remains
Kind: parser gap, fix-and-reclaim
Blocked by: —
Relates to: 183 (the attribution pass that named these), 31 (the same class: whole form sections unclaimed), 41/issue-36 (internal-ojs `_SUM` alias shim)

## What (measured via /v1/sql, 2026-08-13, the issue-183 attribution readout)

The still-held `unclaimed-content` bucket (2,829 rows) is dominated by named form-family gaps:

| population | rows | detail shape |
|---|---|---|
| **VEAT notices** | **2,380** | `unclaimed element at /TED_EXPORT/FORM_SECTION/VOLUNTARY_EX_ANTE_TRANSPARENCY_NOT…` |
| defence forms | ~84 | `…/CONTRACT_DEFENSE` 76, `…/CONTRACT_AWARD_DEFENSE` 4, `…/PRIOR_INFORMATION_DEFENSE` 2, `…/CONTRACT_CONCESSIONAIRE_DEFENCE/FD_…` 2 |
| internal-ojs 2008 | 7 | `unclaimed element at /INTERNAL_OJS/CONTRACT_CONCESSIONAIRE_SUM` — the 7 `.en` English originals (issues 84/139/190's protected population!) |
| eForms UBL | ~311 | split off as issue 195 |
| text-era orphan lines | ~45 | `line NN: …` — stray continuation/menu lines, likely honest residue |

## Why this is one issue

All three TED_EXPORT-era populations are the issue-31 class: a form section the rule registry
never claimed holds the whole notice (ADR-0004). The VEAT family (F15, the voluntary ex-ante
transparency notice, 2010–2015 era) is a whole notice TYPE missing from the walker — 84% of the
remaining actionable bucket in one rule gap. The defence rows are siblings in the same registry.
The 7 internal-ojs rows are likely a missing `_SUM` alias (`CONTRACT_CONCESSIONAIRE_SUM` →
its r209 base name) in `crates/ingest/src/internal_ojs.rs`'s SUM_ALIASES table — and fixing
them ALSO retires the issue-190 sibling caveat for good: once these 7 parse, the 154 protected
siblings become guard-accepted duplicates and skipped-by-policy turns truthful.

## What to do

1. Claim `VOLUNTARY_EX_ANTE_TRANSPARENCY_NOTICE` (and the defence sections) in the r208/r209
   rule registry — mine the archived members for the element inventory the same way issue 31
   did; add fixtures from held members; the era walker already handles the surrounding forms.
2. Add the `CONTRACT_CONCESSIONAIRE_SUM` alias + fixture.
3. Gate, deploy, `reprocess unclaimed-content`, verify: bucket 2,829 → ~45 + eForms 311
   (issue 195); then run the sibling marker path for the newly-parsed originals' 154 siblings
   (or verify the reprocess flag pass now marks them via the guard).

## Comments

**2026-08-13 ~02:0x CEST (orchestrator) — the internal-ojs slice is VERIFIED on prod and the
whole 2008 DTD story is terminally closed.** Deploy 69d869f; reprocess job 625 reclaimed the 7
concession summaries (fold 626: 7 tenders); the guarded marker (jobs 627/628: dry-run 154/0
gaps, execute with expect=154) recorded the 154 siblings skipped-by-policy WITH the guard
satisfied. Panel: unparsable-xml 6,495 → 6,341, all four 2008/2010 DTD ledger rows at
outstanding 0, ledger texts updated to the final story. Remaining in this issue: the VEAT
family (2,380) and defence-form (~84) rule mining.
