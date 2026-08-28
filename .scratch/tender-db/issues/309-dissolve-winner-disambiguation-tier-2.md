# 309 — dissolve tier-2 winner disambiguation: rescue the 1,375 skipped orgs

Status: TIERS 2+3 LANDED AND RUN 2026-08-28 (12de6f3; wet run job 1337) —
**924 of the 1,375 rescued** (334,783 winner rows, 300,503 bid-party rows,
41,265 parties, 31,663 mentions, 7,580 tenders; preview-exact). The prod
run also revealed the REAL tier-1 skip mechanism: rounds ACCUMULATE, so a
carried-forward winner row's causing notice never mentions the org at all
— tier 3 (tender-chain-scoped single-(name,country) resolution) covers
that shape. **451 residual** — including the seven flagship mega-orgs
(NIMAT500-503/100, DE123456789/15176, bare-123456789/15566): within one
tender's chain they carry mentions under DIFFERENT names (buyer and winner
both placeholder-keyed), honestly ambiguous to tiers 1-3. Tier 4 below is
the designed fix.
Kind: repair-job refinement
Relates to: 300 Stage 1 (the dissolve whose skip set this is), 259 (the
guard discipline it inherits — and whose section-walk machinery tier 4
reuses).

## Tier 4 (next unit): lot-result origin resolution

`lot_results` rows carry their ORIGIN (tender_id, notice_id, result_key) —
the notice and RES-section where the result was published, independent of
which version's row carried the winner forward. For a winner row
(T, seq, LR): take lot_results[LR].notice_id + result_key; in THAT notice's
parse layer (notice_sections), find the Organization-kind section(s) whose
ancestor chain reaches the RES section (the 259 `nested_org_aliases`-style
walk, PARTY_KINDS families); the condemned org's mention on (notice, that
section) is the winner-side mention — per-ROW precise, no name agreement
needed. Guards: exactly one such section resolving to the condemned org,
else skip (org-atomic as ever). This is the honest per-row answer the
schema always contained; tiers 1-3 remain as cheap fast paths.

## What

The placeholder dissolve's winner repoint resolves through the version's
`caused_by_notice_id` and requires exactly ONE mention of the condemned org
on that notice; any org with an unresolvable winner row is skipped whole.
The first prod dry-run (job 1333): 7,865 condemned, 6,490 dissolvable,
**1,375 skipped** — including flagship exemplars: org 15566 (bare-123456789
DE bucket, 723 multi-mention notices) and likely 15176 and the PL823 org.
The NIMAT flagship (org 211) is clean (0 multi-mention notices) and
dissolves in tier 1.

## The tier-2 rule

`tender_version_parties` rows carry role + the exact (mention_notice_id,
mention_section_id). For a winner row on (T, seq) whose causing notice
holds ≥2 mentions of the condemned org: among those mentions, keep the ones
whose (notice, section) appears in `tender_version_parties` on the SAME
(T, seq) with a winner-family role and organization_id = the condemned org.
If exactly one survives, that mention's re-resolution target takes the
winner row; else still skip. This should rescue most of the 1,375 — a
notice typically mentions a placeholder org twice as buyer + winner, and
the party roles split them.

## Acceptance

- unit test: a two-mention notice (buyer S-1 + winner S-2, both on the
  condemned org) dissolves, the winner row following S-2's target;
- prod dry-run: skipped count falls substantially from 1,375; 15566 moves
  to the dissolved set;
- the tier-1 run's results are untouched (tier 2 only widens).
