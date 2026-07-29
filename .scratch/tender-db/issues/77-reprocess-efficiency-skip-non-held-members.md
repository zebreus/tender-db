# 77 — reprocess re-parses whole packages (slow for sparse quarantine buckets)

Status: implemented + green (team-lead reviews, deploys after the running 1.0 reprocess finishes)
Kind: performance
Relates to: 76 (the reprocess mechanism), 71/72/73/74/75 (the buckets it reclaims)

## Observation (2026-07-29, SDK 1.0 reprocess prod debut)

The issue-76 reprocess (`reclaim_package`) re-reads AND re-parses EVERY member of each
held package, then `reclaim_notice` no-ops the already-parsed ones (branch (a)). For a
SPARSE bucket this is hugely wasteful:

- SDK 1.0 bucket: ~3.4K held members, but spread across 48 packages totalling **29,475
  members** — so it re-parses ~8.6× more members than it reclaims. Some packages are
  ~24K members / ~10 min each; the whole 48-package bucket is estimated **1.5–3h**.
- Extrapolated to the full ~1.8M program (OC text-era, SDK/DE all thin-spread across many
  packages), the reprocess would take **days**.

It is correct + isolated + resumable (validated in prod), just slow.

## Fix — parse only the held members per package

Before parsing a package, look up the held member set for it from `quarantine`
(the `(fetch_id, member_path)` rows matching the bucket's reason/detail_like/profile), and
skip members whose `member_path` is not in that set — extract + dispatch + parse ONLY the
held members. Parsing is the expensive step (the ~10min packages are parse-bound), so this
drops per-package cost to ~(held/total), ~8× for SDK 1.0 and far more for the denser text
buckets. Reading the package tar is unavoidable (the held members' bytes live there), but
we can seek to just the held members' entries.

- The held-set lookup is a bounded per-package query (indexed on fetch_id).
- Keeps the same dispatch+parse producer for the held members → parsed layer stays
  byte-identical (only non-held members, which were no-ops anyway, are skipped).
- Preserves isolation + bounded-memory + resumability (unchanged control flow).

## Validation

- Byte-identity gates stay green (skipped members were AlreadyParsed no-ops).
- A reprocess of a sparse bucket completes in ~(held/total) of the current time.
- Same reclaimed count as the un-optimised path (no member that would reclaim is skipped —
  only already-parsed non-held members are).

## Note

Not blocking correctness — the current reprocess recovers everything, just slowly. But it's
the difference between hours and days for the ~1.8M program, so worth doing before the big
(OC 577K, SDK 1.7 344K, DE 218K) buckets run.
