# 33 — Dashboard: pipeline funnel + honest re-walk display

Status: needs-verification

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

## Fix (2026-07-21)

Part 2 (honest re-walk display): `JobProgress` gains a `duplicates` counter,
surfaced from `run_process`'s progress callback (the processor already returns
it per package). The dashboard's running-job line now, when `duplicates >
notices` (a re-walk writes mostly dedups), reads "Re-walking already-ingested
packages — N dup, M new" instead of a bare 0.0 notices/s; the member progress
bar above shows it is very much alive. No new DB load — pure in-memory counters.

Part 1 (pipeline funnel): a per-source "Pipeline" panel — published (ground
truth, TED) → fetched (distinct package periods + range, with "fetch complete ✓"
when the latest fetched period is in the current year) → processed (notices) →
projected (Tenders). Data: `fetch_registry_summary` (per-source count + MIN/MAX
period over the tiny fetch registry) and `tenders_by_source`, plus the notice
counts and ground truth `measure` already gathers. All run in the background
refresher (issue 20 part 3), so the request path stays scan-free — no new load.
The per-year Coverage grid is unchanged, below the funnel.

Tests: `store::pipeline_stage_queries_summarise_per_source` (distinct periods +
range + per-source tenders); model classifier + coverage tests unchanged. Full
app suite + wasm check + clippy green.

Acceptance met: a re-walk states in words what it is doing; the funnel shows
fetch-complete vs processing position at a glance; no new DB load.
