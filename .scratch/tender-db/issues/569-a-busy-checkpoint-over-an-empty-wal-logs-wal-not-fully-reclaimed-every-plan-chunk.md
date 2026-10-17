# 569 — a busy checkpoint over an empty WAL logs "WAL not fully reclaimed" on every plan chunk

Status: done (pending deploy) — 2026-10-17, built with 568 (wf_0950d693-9da). **The filed premise was wrong** (found by
the review, confirmed on the box): turso 0.7.2 reports `wal_frames=0 checkpointed=0` for EVERY TRUNCATE checkpoint
(a busy one leaves both registers NULL; a completed one truncates to 0), so "busy over an empty WAL" was never what
the line showed. Project 2590's `.diag.log` (`/data/db/tender-db.db.diag.log`) shows the WAL climbing from 1,459 MB
(chunk 1095) to 2,026 MB (chunk 1455) through the plan phase: busy=true was the TRUE signal (the WAL pinned and
growing, issue 304's shape), and only the frame fields were meaningless. Fix: `plan_checkpoint_unreclaimed(busy,
wal_bytes) = busy && wal_bytes > 0`, the journal and diag lines print `wal_bytes=`; pinned by
`a_busy_checkpoint_over_an_empty_wal_is_reclaimed` and a real-engine store test
(`checkpoint::tests::a_busy_truncate_reports_no_frame_counts_even_over_a_pinned_wal`, which fails if a turso upgrade
starts reporting real counts). The line still fires on every plan chunk while the plan pins the WAL — now with the
size, which is the signal it exists for.
Kind: operability (log noise that hides the signal it exists for)
Relates to:
- 63 (the plan-phase checkpoint and its diag trace);
- 305 (the INCREMENTAL → FULL fallback that makes a whole-corpus plan routine for field refolds).

## Verify

    ssh root@zebreus.click 'journalctl -u tender-db --since "-1h" --no-pager | grep -c "WAL not fully reclaimed"'

- **done (revised)**: the line fires when a busy checkpoint leaves a non-empty WAL file and prints its size in bytes;
  it no longer prints turso's always-zero frame counts as if they were readings.
- **open (2026-10-17)**: project 2590 printed `[project] plan checkpoint after chunk N (notices=…): busy=true
  wal_frames=0 checkpointed=0 — WAL not fully reclaimed` for every one of its ~1,494 chunks (one per 10,000 notices);
  550's project 2531 on 2026-10-15 did the same from chunk 1.

## What was found

`crates/ingest/src/project.rs` ~2425, in phase 1's per-chunk checkpoint (`PLAN_CHECKPOINT_EVERY = 1`):

    let unreclaimed = c.busy || c.wal_frames > c.checkpointed + 20_000;

A TRUNCATE checkpoint reports `busy` whenever a reader holds a snapshot — on the live box the public readers always do —
even when there is nothing to checkpoint (`wal_frames = 0`). So `unreclaimed` is true on every chunk of a whole-corpus
plan, and both the journal line and the `.diag.log` line fire ~1,500 times per full projection. The comment above it
says the line exists to tell a reader-pin (busy) from a climbing WAL (throughput); with every chunk flagged, a real
busy-with-frames episode is indistinguishable from the noise.

Proposed: `let unreclaimed = (c.busy && c.wal_frames > 0) || c.wal_frames > c.checkpointed + 20_000;` — and a unit test
over the predicate (extract it as a small pure fn: busy over 0 frames → false; busy over frames → true; not busy over a
backlog > 20,000 → true). No behaviour other than logging changes.
