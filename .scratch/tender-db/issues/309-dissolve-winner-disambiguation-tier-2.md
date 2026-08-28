# 309 — dissolve tier-2 winner disambiguation: rescue the 1,375 skipped orgs

Status: open (filed 2026-08-28 from the first prod dry-run of the issue-300
dissolve)
Kind: repair-job refinement
Relates to: 300 Stage 1 (the dissolve whose skip set this is), 259 (the
guard discipline it inherits).

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
