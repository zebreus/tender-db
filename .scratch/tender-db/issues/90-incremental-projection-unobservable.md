# 90 — the incremental projection is unobservable between grouping and completion

Status: FIXED 2026-08-02 (proj-fix) — per-stage timings + a per-batch fold heartbeat added to
`project_incremental_chunked`. Awaiting team-lead diff review + deploy.
Kind: operability / diagnosability
Blocked by: —
Relates to: 58 (incremental projection), 59 (Phase-1 heartbeat — the same fix, for the full path),
63 (the `.diag.log` channel), 91 (incremental Phase-2 read amplification), 85 (the re-fold this hid)

## Symptom

`project_incremental_chunked` printed **nothing** between its start and its single closing summary
line. Every intermediate stage — pass-1 identity, the touched-Tender expansion, the pass-2 plan build,
grouping, retirement, and the whole Phase-2 fold+apply loop — was silent. The only observable events
were the `[project] group step …` lines that `build_plan_groups` prints from the store layer.

The full projection has had heartbeats since issue 59 (`Progress::Planning` / `Grouped` / `Applying`,
logged to stderr by `project_with_batch`). The incremental path never called `on_progress` at all and
took no progress sink.

## Why it mattered

Two multi-hour stalls had to be diagnosed without it, both blind:

- **2026-07-30**, the 1.73M reclaim.
- **2026-08-01**, the eForms-DE 1.x re-fold (issue 85): the trailing `project rebuild=false` ran
  ~7h24m (5h22m CPU), of which the last **5h07m was a single silent CPU-bound stretch** after the
  `group step analyze` line at 16:47:08. It was killed at 21:54 with nothing committed.

With no stage output the only evidence available was external: `/proc/<pid>/io` counters, WAL size,
`tursodb-temp` size, and the changes cursor. That was enough to bound the stall (post-grouping,
pre-first-write, in-RAM, single-threaded) but **not** to name the stage — which cost a full
diagnosis cycle and produced two wrong hypotheses before measurement eliminated them (see 91).

## Fix (landed)

In `project_incremental_chunked`, a local `stage(label)` closure prints elapsed seconds at every stage
boundary, plus a one-line change-set announcement at the start and a per-batch heartbeat inside both
Phase-2 folds:

```
[project] incremental: 218635 changed notices
[project] incremental stage pass-1 identity (170123 new keyed keys): …s
[project] incremental stage touched expansion (218635 touched Tenders → 402118 planned notices): …s
[project] incremental stage pass-2 plan build (… mentions resolved): …s
[project] incremental stage grouping (… Tenders, … islands): …s
[project] incremental stage retire regrouped (… retired): …s
[project] incremental phase 2: Buckets { shards: None } over 402118 planned notices
[project] incremental fold: …/… Tenders applied
[project] incremental stage phase 2 fold + apply: …s
```

stderr rather than `db.log_diag`: the `[project] group step …` lines from this same path reached
journald in both stalled runs, so the channel is proven on this path (unlike the Phase-1 case that
motivated the `.diag.log` in issue 63).

The counts are deliberately part of the labels — `touched Tenders`, `planned notices`, `Tenders`,
`islands` — because the stall diagnosis needed exactly those numbers and had to estimate them from
`CREATE INDEX` timings instead.

## Acceptance

- A stalled incremental run names its own stage in the log, with no external process inspection.
- The planned-notice count (which now selects the Phase-2 fold, issue 91) is visible in the log.
