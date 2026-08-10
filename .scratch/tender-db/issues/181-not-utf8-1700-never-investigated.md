# 181 — the 1,700 `not-utf8` rows were flagged suspected and never investigated

Status: needs-triage
Kind: data-quality investigation (suspected-gap bucket)
Blocked by: —
Relates to: 30 (classified it SuspectedGap rather than assumed-benign), 137 (measured: 1,700 rows, 0 reclaimed)

## Why

`quarantine_class` deliberately flags `not-utf8` as `SuspectedGap` — "small uncertain reasons are
flagged too rather than assumed benign" (issue 30). That flag is a promise to investigate, and
nothing has: 1,700 rows, zero reclaimed, no issue, no diagnosis, since the reason first appeared.
It is the largest still-held population on the dashboard with no owner at all
(public `/api/dashboard`, 2026-08-10: still-held `not-utf8` = 1,700).

## What

Sample the bucket (bounded `/v1/sql` + archive bytes, the issue-141 method) and answer:

1. What encoding are they actually in? Legacy text-era files in Latin-1/CP1252 would be real
   notices recoverable with a transcode step — a parser fix and a reclaim.
2. Or are they binary/truncated members — benign-by-evidence, to be reclassified with the reason
   carrying the proof (issue 30's bar)?

Either way the outcome gets a ledger row; a bucket this old should not be answerable only by
re-deriving it.
