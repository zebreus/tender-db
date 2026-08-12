# 190 — the reprocess-time skip flag has no parsed-original guard: it swept the 154 protected siblings into skipped-by-policy

Status: RESOLVED-VERIFIED (2026-08-12) — guard landed in flag_skipped_members, repair job restored all 154 on prod, panel reads outstanding 154 again
Kind: data-quality regression (bookkeeping, not data loss)
Blocked by: —
Relates to: 84 (defined the guard and the 154's protected-outstanding status), 139 (the reprocess runs that did the sweeping)

## What happened

Issue 84's one-time marker was guarded row by row: a 2008 non-English sibling is only marked
skipped-by-policy when its English original is **present AND parsed** — the 154 siblings whose
originals are the 7 still-held `.en` rows were excluded by construction and given their own
ledger row ("2008 siblings whose original did not parse", explicitly OUTSTANDING).

The reprocess path has a second, unguarded flagger: `reclaim_package` collects every held member
the dispatch policy declines and `Db::flag_skipped_members` stamps `skipped_at` on ANY still-
unskipped row matching `(fetch_id, member_path)` — no original-parsed check. The 2026-08-12
DTD-bucket reprocess (job 612) walked the 2008 monthly, declined all 593,010 siblings, and the
flag pass swept the 154 in with the rest: the panel's sibling-gap row now reads outstanding 0,
skipped 593,010, although nothing about their originals changed (still held, unclaimed-content).

Issue 84's line was: an unparsed original means the sibling is potentially the only readable copy
— marking it a duplicate records real loss as policy. The flag pass silently disagrees with the
marker's own guard.

## What

1. Guard `flag_skipped_members` (or its caller) with issue 84's predicate: only flag a declined
   sibling when the English original is present and parsed; leave the rest outstanding with the
   attempt recorded (issue 87 shape: `last_attempt_at`, not `skipped_at`).
2. Repair pass: clear `skipped_at`/`skipped_reason` on the 154 (identifiable as the DTD-detail
   sibling rows whose original's notice is unparsed — `count_skipped_sibling_gaps`' predicate).
3. Re-measure: the sibling-gap ledger row must read outstanding 154 again (or whatever the count
   is after issue 139's fixes reach the 7 originals — if the originals eventually parse, the
   sweep becomes CORRECT and the row drains for real).

Note the interaction with 139: if the 7 `.en` originals are reclaimed (their current failure is
`unclaimed-content`, not DTD), the 154's guard passes and skipped-by-policy is then the truthful
terminal state. Order the repair after 139's endgame so the flags settle once, not twice.
