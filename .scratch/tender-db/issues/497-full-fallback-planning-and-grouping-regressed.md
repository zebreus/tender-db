# 497 — full-fallback planning grew 90 % since August and grouping 5×, against 4.6 % more notices

Status: ready-for-agent — filed 2026-10-08 from the refold diagnosis (`wf_805a60a7-d9b`, `../495-refold/`).
NEXT: unit 1, find which change moved each number.
Kind: performance / projection planning (`crates/ingest/src/project.rs`)
Relates to: 58 / 179 (the planning half of the full fallback, deliberately left open), 192, 305 (the
closure cap), 495, 496

## What was measured

Read from the `[project]` journal lines saved in `../495-refold/p2_*.txt`:

- **Plan** (phase 1 of the whole-corpus fallback):
  - about 5,213–5,357 s in August (the first run was 5,927 s);
  - 10,085.0 s in both job 2044 (2026-10-07) and job 2067 (today).
  - That is about +80 min (+90 %), while the notice count grew only about 4.6 %.
- **Grouping:**
  - 312.9 s in job 1616 (2026-09-28);
  - 1,613.7 s in job 1974 (2026-10-04);
  - 1,482 s in job 2044 and 1,468 s in job 2067.
  - That is about 5× in one week. No issue covers it.

Small fixes pay this too. Job 2031 (refold-fields, 191k notices) took 5 h 05 m, because it fell back to
the whole corpus ("legacy closure exceeds cap (622004 > 500000)").

Two oddities:

- The 1,489 per-chunk TRUNCATE checkpoints during job 2067's planning all returned busy and reclaimed
  nothing. The WAL stayed about 2.2 GB, the same as in job 2044. They do nothing as placed.
- The two identical 10,085.0 s plan times are real: the journal shows 2 h 48 m 06 s for each.

## Units

1. **Attribute.**
   - Bisect the grouping jump between job 1616 (09-28) and job 1974 (10-04) against the commits deployed
     in that week. Candidates are the identity and weld work of issues 479, 481 and 486, which add keys.
   - Do the same for planning since August: mention resolution (issue 434's refresh and re-bind
     accounting) and the per-chunk checkpoints.
2. **Fix** what unit 1 names. Expected gain: about 20–90 min per full fallback.
3. **Drop the planning checkpoints**, or move them to a point where they can succeed, if unit 1 confirms
   they never reclaim anything.
