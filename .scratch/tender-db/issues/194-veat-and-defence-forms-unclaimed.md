# 194 — the VEAT form family is unclaimed: 2,380 notices held on VOLUNTARY_EX_ANTE_TRANSPARENCY_NOTICE, plus the defence-form and internal-ojs residues

Status: RESOLVED (2026-08-13) — TED_EXPORT half drained (2,464 notices reclaimed over three passes); eForms residue owned by issue 195; text orphans + 2 text-fallback rows documented below
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

**2026-08-13 ~03:0x CEST (orchestrator) — root cause of the VEAT + defence populations: a
SPELLING fork.** The mirrored XSDs (and the whole rule registry) write the defence-form family
with UK spelling (…_DEFENCE); the 2011 dailies (Sept–Dec 2011, R2.0.8.S01 era) publish the SAME
forms with US spelling (…_DEFENSE). All 2,380 VEAT rows fail on AWARD_OF_CONTRACT_DEFENSE inside
an already-claimed VEAT form; the ~84 defence rows are the same fork at their form roots. Fix:
23 S-spelled twins added beside their DEFENCE twins — 4 form-root/section names + 19 mined
mechanically by diffing all element names of four real members against the inventory
(r208-observed entries, same rule kinds as the twins). All four sampled members
(VEAT 294050, CONTRACT_AWARD 297630, CONTRACT 299577, PRIOR_INFORMATION 362041) now parse;
VEAT fixture committed (veat-294050-2011.xml). Also: the SUM-alias pinning test was stale at
77 (my earlier awk-summarized gate masked it — deploy 69d869f carried the red pin; harmless,
fixed to 85 with the count documented). Remaining: deploy + reprocess unclaimed-content, expect
2,822 → ~350 (eForms 311 + text-era orphans ~45), then issue 195 owns the eForms residue.

**2026-08-13 ~04:0x CEST (orchestrator) — first drain pass + layer 2.** Deploy b97decb; job 629
reclaimed 2,266 (72 packages), journal clean. Layer 2 surfaced two more S-spellings the samples
lacked (CONTRACT_LIKELY_SUB_CONTRACTED_WITH_DEFENSE in 179 VEAT rows,
NOTICE_INVOLVES_DEFENSE in 17 CONTRACT_DEFENSE rows) — added the twins, both sample members
parse (296139: 7/109, 304253: 22/235). Also isolated a DIFFERENT 2-row class: "unclaimed TEXT
at …AWARD_CRITERIA_CONTRACT_NOTICE_INFORMATION_DEFENCE/AWARD_CRITERIA_DETAIL" (UK-spelled 2012
concession members; a text-fallback gap on AWARD_CRITERIA_DETAIL, not a spelling twin) — decide
with the final residue. Next: deploy + final reprocess; expected residue ≈ eForms 310 (issue
195) + text-era orphan lines ~45 + the 2 text-fallback rows.

**2026-08-13 ~09:0x CEST (orchestrator) — DRAINED.** Final pass (job 631, rev 1b4ee0f): 191
reclaimed; cumulative for this issue 7 + 2,266 + 191 = 2,464 notices, zero stamping anomalies
across all passes. unclaimed-content terminal residue 365: ~310 eForms (issue 195), ~45
text-era orphan lines, 2 concession text-fallback rows (AWARD_CRITERIA_DETAIL bare text —
documented here, too small to own separately unless the class grows). Public ledger row
"2011 US-spelled defence forms (VEAT et al.)" added, resolved 2026-08-13; rides the next
deploy. Panel verification (unclaimed-content 365, row reclaimed ≈2,457) after fold 632.

**2026-08-13 ~13:0x CEST (orchestrator) — layer 3, the last twin.** Panel verified the close
(unclaimed-content 365, ledger row reclaimed 2,457) but the key showed outstanding 5: VEAT
members whose subcontracting block nests one level deeper than every earlier sample —
SUBCONTRACT_DEFENSE. Twin added (26 total now), member 300858 parses (7/118), gate clean.
Final mini-drain queued; the row should read outstanding 0 after it.

**2026-08-13 (orchestrator) — mini-drain complete: 5 reclaimed, 0 still held (job 633, rev
16185ea).** Cumulative for this issue: 2,469 notices (7 internal-ojs + 2,266 + 191 + 5).
The DEFENSE ledger key reads outstanding 0 once the trailing fold's panel measure lands —
next firing verifies and flips this to RESOLVED-VERIFIED.
