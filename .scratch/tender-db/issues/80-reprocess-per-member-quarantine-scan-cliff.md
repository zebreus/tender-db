# 80 — reprocess per-member CPU cliff: unindexed quarantine.notice_id → 2.4M-row scan/member

Status: fixed + green (team-lead deploys before re-running the dense big buckets)
Kind: performance / correctness-of-scale
Relates to: 76 (reprocess mechanism), 77 (parse-only-held)
Found: 2026-07-29, SDK 1.7 reprocess prod run, package "fetch 18" (69,930 members)

## Symptom (run-driver)

The reprocess cliffed on a huge dense package: ~1 member/second, one core pegged
(105% CPU, io ≈ 0 — algorithmic), RSS climbing +287M/60s (OOM trajectory). SDK 1.0
(sparse packages) and SDK 1.7's first 8 (small) packages flew on the same binary;
only the 70k-member package cliffed.

## Root cause

`reclaim_notice_tx`'s parse-level branch (the case where the notice row already
exists — every `unknown-customization` reclaim) flags the reclaimed member with:

    UPDATE quarantine SET reprocessed_at = ? WHERE notice_id = ? AND reprocessed_at IS NULL

`quarantine` was indexed on `reason` and `UNIQUE(fetch_id, member_path, content_hash)`
but **NOT on `notice_id`**. So that UPDATE full-scanned all ~2.4M quarantine rows
**every reclaimed member** — a ~1 s CPU scan, cached (io ≈ 0). Cost is
`O(held_members × 2.4M)` per package: negligible for sparse packages (1.0: 2-3
held), catastrophic for a 70k-member dense one (~70k scans ≈ 19 h). The RSS climb is
secondary: reclaim writes commit to the WAL but the reprocess only checkpointed
per-package (at the end), so a 70k-member package grew the WAL its whole length.

The profile-level flip (`WHERE fetch_id = ? AND member_path = ?`) was already fine —
it seeks via the `UNIQUE(fetch_id, member_path, …)` index prefix.

## Fix

1. **`CREATE INDEX quarantine_notice_id ON quarantine(notice_id)`** — the per-member
   flag now seeks its row instead of scanning 2.4M. Kills the CPU cliff. One-time
   ~seconds index build on first open of the prod DB (like `quarantine_reason`).
2. **Intra-package checkpoint** — `reclaim_package` TRUNCATE-checkpoints every 5,000
   members walked, bounding WAL/RAM on a huge package (no OOM even mid-package).

## Validation

- `reclaim_flag_seeks_the_notice_id_index` (EXPLAIN QUERY PLAN): the flag SEARCHes
  the `quarantine_notice_id` index, never SCANs.
- No parse-output change → byte-identity holds; full store+ingest+app suites green
  incl. the projection golden/equivalence/resume gates.

Deploy before re-running the dense big buckets (1.7/1.10 both have huge packages).
