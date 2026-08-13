# 193 — record_reclaim_attempt misses the rows of identity-less text records: 77 relabels stamped nowhere

Status: RESOLVED (2026-08-13) — content-hash fallback landed with test; verify on the next unclaimed-content reprocess (77 relabels should land)
Kind: bookkeeping defect (small, no false resolution)
Blocked by: —
Relates to: 87 (the relabel contract), 139 (the same address-mismatch family, at scale), 183 (the pass that measured it)

## What happened (measured, job 623, 2026-08-12)

The unclaimed-content reprocess reported "still held by current reason: … missing-publication-id
77" — 77 records that now fail BEFORE an identity exists, so they re-arrive as
`Record::Quarantine` and their attempt is recorded via `Db::record_reclaim_attempt(fetch_id,
member_path, …)`. After the pass, `by_reason` still reads missing-publication-id 108 (unchanged)
and the 77 rows still sit under unclaimed-content: the relabel UPDATE matched zero rows.

Likely mechanism (same family as issue 139's second act): the walker's `QuarantineRecord`
member_path for a failed text-era record differs from the stored row's — text rows carry a
`#<ordinal>` suffix per record, a dispatch-level failure names the member file without it (or
the reverse), so the exact `member_path = ?` address misses. Fail-safe direction: the rows stay
OUTSTANDING under their stale reason — nothing is falsely resolved — but the attribution issue
87 promises (current failure recorded on the row) silently does not happen for this class.

## What

1. Reproduce in a unit test: a text-era row with `member_path 'file.zip#3'`, a reclaim attempt
   arriving as `Record::Quarantine` with `member_path 'file.zip'` (or vice versa — read the
   walker to fix the direction), assert the attempt stamps.
2. Fix the addressing: `record_reclaim_attempt` should match the member file the same way the
   held-set builder does (`member_file()` strips the ordinal), e.g. also update rows whose
   ordinal-stripped member_path equals the record's — or carry the ordinal into the
   QuarantineRecord.
3. Consider the same zero-stamp journal line the reclaimed path now has: an attempt recorded
   on zero rows is this exact defect's signature.
4. Re-run the unclaimed-content reprocess afterwards; the 77 should relabel to
   missing-publication-id (by_reason 108 → 185, unclaimed-content 2,829 → 2,752).

77 rows; bounded; not urgent. The 2,829 remaining unclaimed-content rows DID relabel correctly
(their current details are on the rows) — this class only covers records that lose their
identity entirely under the current dispatcher.
