# 261 — orphaned identifier-less organizations: the residue re-parse churn left behind

Status: needs-triage — filed 2026-08-21 (owner) out of issue 234's close-out numbers.
Kind: canonical layer hygiene / sizing first
Blocked by: —
Relates to: 234 (whose merge collapsed the duplicates but not the orphans), 247 (the re-parse
clear that creates them), 103 (the same "rows nothing references" class, one layer down)

## What

A re-parse CLEARS a notice's mentions and party rows, then re-resolves. Pre-merge, every
re-resolved identifier-less mention minted a FRESH provisional org, leaving the OLD org with zero
references — and the merge (issue 234, job 296) only collapses `(name_norm, country)` DUPLICATE
groups, so an orphan whose name became unique again survives as a singleton. The scale of the
churn is visible in issue 234's close-out: the org table grew 24.6M → 30.5M in three days of
campaign re-parses.

Post-merge state (2026-08-21): 12,273,530 orgs, 11,115,712 identifier-less. Some unknown slice of
those singletons has NO mention, party, bid-party, or winner row pointing at it — dead weight that
inflates every org count and the provisional share (90.6 %).

## First step: SIZE it (offline — over the /v1/sql cap)

    SELECT COUNT(*) FROM organizations o
     WHERE o.identifier IS NULL
       AND NOT EXISTS (SELECT 1 FROM organization_mentions m WHERE m.organization_id = o.id)
       AND NOT EXISTS (SELECT 1 FROM tender_version_parties p WHERE p.organization_id = o.id)
       AND NOT EXISTS (SELECT 1 FROM tender_version_bid_parties b WHERE b.organization_id = o.id)
       AND NOT EXISTS (SELECT 1 FROM tender_version_result_winners w WHERE w.organization_id = o.id)

~11.1M rows × up to 4 indexed probes (all four org indexes exist). Run when the box is quiet —
sqlite3 on a snapshot, or a small admin measurement. If the count is large (likely, given +5.9M
table growth from churn), step 2 is a delete sweep in the merge job's image: batched,
checkpointed, `removed` change rows, dry-run default. If small, close as not-worth-a-job.

## Guard rails

- Orphanhood must be checked against ALL FOUR referencing tables, not just mentions — a party row
  can outlive its mention only if the clear left it, but check rather than assume.
- Do NOT touch identifier-BEARING orgs: canonical rows are the layer's spine, and an unreferenced
  one may be re-referenced by the next daily fold's `org_of` preload semantics.
