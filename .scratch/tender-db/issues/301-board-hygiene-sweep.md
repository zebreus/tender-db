# 301 — board hygiene: stale status lines + CONTEXT.md drift

Status: BACKLOG (filed 2026-08-26; flagged by the 291 deferred-inventory sweep)
Kind: operational (the board is the source of truth — keep it true)

Verified-stale items to correct:
- Issues 280/281/282/284/285 still read "FIXED in working tree, awaiting
  gate+deploy" — they gated and deployed in the 2026-08-26 batches (revs
  c20b7c6..5036a09 line). Update each to its deployed rev.
- Issue 71 reads "IN PROGRESS … pending the final rebuild" but the rebuild ran
  (issue 15, verified 2026-08-16). Issues 221/226/235 read "awaiting deploy"
  across many landed deploys — verify each on prod, then close.
- Issue 23 (off-box user-state copy) predates the 2026-08-23 re-decision on
  issue 170 (NO backups) — reconcile or close as superseded.
- CONTEXT.md still calls service.bund.de "a possible later stress-test Source";
  the deep-dive verdict is "do not ingest" (service-bund-de.md §9) — correct it.
One firing, no code: read each on prod where needed, fix the lines, done.
