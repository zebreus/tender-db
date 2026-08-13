# 196 — monthly 2026-06 fetch: 21 inner dailies held whole as not-utf8 members

Status: needs-triage
Kind: bookkeeping / walker attribution
Blocked by: —
Relates to: 180 (found while attributing the not-utf8 remainder)

## What (measured via /v1/sql, 2026-08-14)

Fetch 2 (`ted monthly 2026-06`, fetched 2026-07-19) holds 21 outstanding `not-utf8`
quarantine rows whose member_path is a whole inner daily — `06/20260601_2026103.tar.gz`
etc., no inner member, detail NULL. The walker recorded the nested daily tarballs as raw
members (binary bytes → not-utf8) instead of recursing into them.

NOT a coverage gap: the same days are ingested (e.g. member_path LIKE %20260601% → 4,058
notices) — presumably via the daily fetches — and the coverage grid is green. But 2024/2025
monthlies (fetch 25/27/32…) DO recurse their inner dailies fine, so why did fetch 2 record
the dailies as members? Hypotheses: (a) an early process run before nested-daily recursion
handled this monthly's inner layout, rows now stale — a reprocess would either recurse or
re-hold them; (b) the 2026 monthly's inner naming/layout differs enough to miss the
recursion rule.

Note: the issue-180 `reprocess not-utf8` pass (job 647) did NOT touch these rows — its
"still held" tally showed only the 22 COR rows — so whole-package members may be invisible
to the reprocess pass's member loader. Whatever resolves them must handle that.

## What to do

1. Check the walker/process code for how nested `.tar.gz` members inside a monthly are
   recursed and why `06/…tar.gz` in fetch 2 wasn't.
2. If stale-rows: make a reprocess pass able to address whole-package members, or resolve
   the rows another sanctioned way (they are duplicates of daily-fetch content).
3. If layout: fix the recursion and reprocess; verify no notice from the monthly is
   actually missing vs the daily fetches (spot ND diff for one day).
