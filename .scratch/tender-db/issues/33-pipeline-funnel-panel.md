# 33 — Dashboard: pipeline funnel + honest re-walk display

Status: ready-for-agent

User feedback (Lennart, 2026-07-21): during the backfill re-walk the
dashboard showed "0.0 notices/s" for an hour — indistinguishable from a
hang — and nothing answers "is downloading done? are we just
processing?" without ssh.

Two parts:
1. **Pipeline funnel panel**: per source, the stages as a funnel —
   published (ground truth where known) → fetched (packages/periods in
   the archive registry, with an explicit "fetch complete ✓" when the
   full range is on disk) → processed (notices) → projected (tender
   versions). The existing coverage grid stays (per-year detail); the
   funnel is the at-a-glance "which stage are we in".
2. **Honest job progress during dedup**: when a process job is walking
   members but writing ~0 notices (re-walk), label it as such —
   "re-walking already-ingested packages (N members/s, M dup)" instead
   of a bare 0.0 notices/s. The supervisor already tracks member
   counters and dup counts; surface them.

Acceptance: during a re-walk the dashboard states what is happening in
words; the funnel shows fetch complete vs processing position at a
glance; no new DB load (reuse existing counters + registry queries via
the reader pool / TTL cache).
