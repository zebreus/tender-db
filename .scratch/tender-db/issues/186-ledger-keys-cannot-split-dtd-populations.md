# 186 — the ledger join key is too coarse: four DTD rows display one blended count instead of their 1,898/154/7 split

Status: resolved — landed 2026-08-10 (member_path_like/unlike narrowers, five DTD rows keyed, tests); shows on prod after the next deploy
Kind: dashboard correctness (ledger join granularity)
Blocked by: —
Relates to: 40 (ledger design), 84 (named the three populations), 139 (owns the 1,898)

## Why

`LedgerEntry` narrows a category by `(reason, profile?, detail_like?)` only. The four
"XML with DTD detected" rows — the resolved 2008 siblings, the 154 siblings whose original never
parsed, the 7 English originals, and the 1,898 non-sibling 2010-03 rows — all share the identical
key, so the live join shows every one of them `reclaimed=26,948 / skipped=592,856 /
outstanding=2,059` (public `/api/dashboard`, 2026-08-10). The 1,898/154/7 split that issue 84
fought to establish lives only in the rows' prose; the numbers next to each narrative are wrong
for every row individually, and a reader cannot see the 2010-03 population shrink when issue 139
resolves it.

## What

Add an optional `member_path_like` narrower to `LedgerEntry` and thread it through
`Db::quarantine_resolution` — the populations are distinguishable by path shape (issue 84/139:
2008 siblings carry a 2-letter language code before `.xml`; the 2010-03 rows end `.xml` with no
language code; the 7 originals carry `.en`). Then key the four rows so each shows its own count.
The predicate stays a bounded scan on the background refresher, same as today. Alternative if
path-LIKE proves ambiguous: a `fetch_id` narrower (the 1,898 are one fetch, TED monthly 2010-03).
