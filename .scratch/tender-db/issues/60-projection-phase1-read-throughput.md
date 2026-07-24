# 60 — Projection Phase-1 read throughput is ~30 MB/s (slow)

Status: ready-for-agent
Severity: MEDIUM (makes every full projection multi-hour; complements 58)

Observed 2026-07-24 during the first-ever full projection rebuild on prod
(rev 20767e8): Phase-1 (stream parsed layer + resolve mentions + build plan)
read ~183 GB over ~2h at a steady ~25-31 MB/s. Memory bounded (~1.9GB, no
swap), CPU ~50% — it is I/O-bound, not CPU/memory-bound. 30 MB/s is far below
what the VPS disk should sustain, so the bottleneck is likely the ACCESS
PATTERN, not raw disk bandwidth.

Suspected cause: parsed_chunk / the per-notice parsed re-read fans out into
many small scattered reads across notice_sections + each value table by
id-range, rather than large sequential scans — so it's seek/È-bound and turso
per-statement overhead dominates. (The new code also reads the parsed layer
~twice: Phase-1 + Phase-2 scattered re-reads.)

This is DISTINCT from and COMPLEMENTARY to:
- issue 58 (incremental: don't re-read the WHOLE corpus each daily run) — cuts
  the AMOUNT of work.
- issue 59 (bounded/flat memory) — already done.
Issue 60 is about making the bytes it DOES read stream faster.

Directions to investigate (reproduce-first, measure MB/s):
- Larger READ_CHUNK / batch the per-notice value reads into fewer, larger
  ranged scans; ensure the value-table queries hit an index that yields
  sequential id order.
- Avoid the double read of the parsed layer if Phase-2 can reuse Phase-1's
  stream (memory permitting under 59's bounded design).
- Check turso statement/prepare overhead per chunk; reuse prepared statements.
- Confirm it's not fsync/WAL-checkpoint interleaving stalling reads.

Acceptance: full-corpus Phase-1 read throughput materially higher (target a
few×), measured; projection output unchanged (equivalence tests still pass).
