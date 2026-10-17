# 569 — a busy checkpoint over an empty WAL logs "WAL not fully reclaimed" on every plan chunk

Status: ready-for-agent — filed 2026-10-17 from the hourly audit (project 2590, issue 567's drain). Next: the one-line
predicate change below plus a unit test, shipped with the next code deploy. Small; land it beside other work in
`crates/ingest/src/project.rs` (not while another agent holds that file).
Kind: operability (log noise that hides the signal it exists for)
Relates to:
- 63 (the plan-phase checkpoint and its diag trace);
- 305 (the INCREMENTAL → FULL fallback that makes a whole-corpus plan routine for field refolds).

## Verify

    ssh root@zebreus.click 'journalctl -u tender-db --since "-1h" --no-pager | grep -c "WAL not fully reclaimed"'

- **done**: during a project's plan phase the line appears only when the checkpoint left frames behind (or was busy with
  frames in the WAL); a busy result over an empty WAL logs nothing to the journal (the every-16-chunks diag trace stays).
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
