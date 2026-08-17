# 65 — surface projection/big-job progress on the dashboard + /admin/jobs

Status: UNITS 1+2 DEPLOYED 2026-08-17 (unit 1 `ccd606f` in rev `fddecd0`; unit 2 `3fd6201` deployed
same day, health green). The phase record exists, the legacy-adjacency sweep reports through it, and
the projection's planning / pre-pass / folding phases now reach the durable record via
`project_observed` (stderr sink composed with the supervisor mapping — one stream, no drift). The
pre-pass reports its aggregate sweep through the new `Progress::PrePass` (shared counter bumped per
chunk, parent polls 2s, deterministic closing tick). Incremental daily deliberately unobserved
(fast, and its scoping logs its own decisions). Unit 4a DEPLOYED same day (`160b3d8`): the jobs panel renders the phase — label with whichever
counts the phase honestly has, bar when done/total both exist, detail line, and an error line when
the reporter's stamp is older than PHASE_SILENT_SECS (300 s), so dead-vs-slow is visible on the
panel too. REMAINING: a `/metrics` phase gauge (needs the supervisor handle in AppState — its own
wiring unit); the reset/index-build phases still show as dead air within `project`. First prod
evidence arrives with the next rebuild/refold, which will show phases on `/admin/jobs`.
**Part of the motivation as written is STALE — see "Correction" before working this.**
Kind: observability / dashboard
Blocked by: —
Relates to: 228 (filed the same complaint from a chunked backfill and deliberately deferred the
field this issue proposes — "needs a supervisor progress field that is not `members_done`"), 94 (put
the pre-pass heartbeat in the journal), 53 (`/metrics`, the trend surface)

## Correction (2026-08-17, owner) — the pre-pass is no longer silent

The claim below that "the multi-hour `write_buckets` pre-pass emits NOTHING at all until the fold
pass starts" was true when filed and is **not true now**. Issue 94 added `PREPASS_HEARTBEAT`
(`project.rs`, 250k notices per shard) plus a shard-layout line at pre-pass start, after hitting the
same wall this issue describes — and its comment records the same reason the bucket files are no help
(`BufWriter`-wrapped, flushed only at the end, so on-disk size stays near zero however far along the
sweep is).

So proposal item 2's parenthetical "add a periodic journal line too" is **already done**, and anyone
starting from the text below would either duplicate it or conclude the code was worse than it is. The
real remaining gap is exactly item 1 of "What's missing": those heartbeats reach **stderr only** and
never enter the durable progress row that `/admin/jobs` and the dashboard read. An operator watching
the journal is fine today; one watching the API still sees dead air.

That gap has a structural cause worth knowing before starting: the pre-pass heartbeat is emitted
inside `write_shard`, on each worker's own thread, and `on_progress` is a non-`Sync` `FnMut` owned by
the parent — so worker progress cannot simply call it. Routing it needs a shared counter the workers
increment (per chunk, not per notice) and a parent that reports from it while the shards run, e.g.
polling `ScopedJoinHandle::is_finished` instead of blocking straight into `join`. Additive and
output-neutral by construction — it only counts — but it touches the projection's hot pre-pass, so it
wants a prod run to verify rather than tests alone.

## Unit 1, landed

`model::ingestion::Phase { name, done, total, detail, updated_at }` on `JobProgress.phase`, set via
`Supervisor::set_phase`, surfaced by `/admin/jobs` through serde with no handler change. `done`/`total`
are optional so a phase whose end is only provable by reaching it still shows movement; `updated_at`
is stamped inside `set_phase` so a stopped reporter cannot pass for a slow one. The legacy-adjacency
sweep is wired: `members_done` keeps its meaning (a count) and the id-window cursor rides in the
phase, which is the split issue 228 asked for and declined to fake.

## Motivation

During the 2026-07-25 full canonical rebuild there was NO way to see progress
from the outside. `/admin/jobs` reported the running `project` job but with all
its progress fields (`packages_done`, `members_done`, `notices`, …) at 0 — those
are wired for fetch/process, not projection. The only signal was the journal
`[project]` lines, and the multi-hour `write_buckets` pre-pass emits NOTHING at
all until the fold pass starts. Operating the rebuild meant sshing in and reading
`/proc/<pid>/io` read_bytes and `du` on the bucket dir to infer how far along it
was. A long, expensive, once-in-a-while job is exactly the one that needs a
progress bar.

## What's missing

- The projection's `on_progress` (Progress::Applying { tenders, total }) goes to
  the journal only; it is not persisted into the job's durable progress row that
  `/admin/jobs` (GET) and the dashboard read.
- No phase model: reset → grouping → **pre-pass (write_buckets)** → fold →
  index-build are invisible. The pre-pass and the end-of-fold index builds (each
  many minutes) show as dead air.
- `/admin/jobs` `current` has no generic "phase + fraction + detail" field for a
  project job.

## Proposal

1. Give a running job a small structured progress record the supervisor updates
   and `/admin/jobs` returns: `{ phase: string, done: i64, total: i64, detail:
   string, updated_at }`. Generic across job kinds (fetch/process/project).
2. Have the projection report each phase into it:
   - reset_tender_layer: "clearing previous layer"
   - build_plan_groups: "grouping" (or "reusing grouping")
   - **write_buckets: "pre-pass" with notices-processed / total** (it already
     loops `parsed_chunk(after_id, N)` — emit after_id-based progress every chunk;
     add a periodic `[project] pre-pass: N/12.39M notices bucketed` log line too,
     so the journal isn't silent for ~2h either).
   - fold pass: "folding" with tenders-applied / total (already computed).
   - index build: "rebuilding indexes" with which index.
3. Dashboard System/Jobs panel: render the running job's phase + a progress bar
   from done/total + detail. Poll the same way the panel already polls health.

## Notes / scope

- Keep it cheap: the progress write must not add per-row overhead — update at the
  existing chunk/batch boundaries only (every 10k-notice chunk / 50k-tender batch),
  not per notice.
- The pre-pass is the worst offender (multi-hour, zero output). Even just the
  periodic journal line in (2) would remove most of the pain; the dashboard bar is
  the fuller fix.
- Relates to issue 61 (projection starves the HTTP runtime): during the fold the
  dashboard/health can be unresponsive anyway, so the progress record should be
  written such that a later GET (once responsive) still reflects the latest phase.
