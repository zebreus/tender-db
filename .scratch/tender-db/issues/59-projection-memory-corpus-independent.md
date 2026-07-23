# 59 — Corpus-independent projection memory: back the grouping plan with disk

Status: ready-for-agent (design signed off 2026-07-24; in implementation)
Severity: MEDIUM (latent wall; the deployed issue-57 fix keeps prod safe meanwhile)
Follow-up to: 57 (bounded per-batch). Relation to 58: see "Coordination".

## Goal

Projection PEAK RSS must be FLAT across corpus size (not just bounded per batch)
and comfortably under 4 GB — target ~1 GB — so the box never needs memory that
scales with the dataset (owner directive, 2026-07-24).

## What still scales with the corpus

After issue 57, Phase 2 holds only one batch, but Phase 1 still builds the whole
grouping plan in RAM: `Vec<Ident>` (project.rs), ~1–1.5 GB at 7.5M notices and
growing as we ingest. That is the one remaining O(corpus) structure and the
latent wall. (A second, smaller residual — the `MentionResolver.org_of` dedup
map, O(distinct identified orgs), ~150–250 MB at 7.5M — is discussed under
"Residual" below.)

## Design: move the plan (and its grouping) onto disk

Replace the in-RAM `Vec<Ident>` + in-RAM union-find with scratch tables in the
main DB (real tables — guaranteed on disk, unlike TEMP whose store is unconfigured
in turso; cleared at the start of each run like `clear_canonical`). turso lacks
`WITH RECURSIVE` (docs/research/turso-capabilities.md), so the legacy transitive
closure is done by **Rust-driven iterative label propagation** over SQL — verified
working in a spike (converged in 2 iterations on a 3-component graph; correlated
`UPDATE` and `printf('%06d', …)` both work in turso).

### Scratch schema (main DB, cleared per run)

```
plan_notice(
    notice_id     INTEGER PRIMARY KEY,
    procedure_key TEXT,        -- BT-04 / sdk01-uuid, or NULL
    legacy        INTEGER NOT NULL,
    ojs_self      INTEGER,     -- encoded (year*10^7 + number), or NULL
    source        TEXT NOT NULL,
    source_rank   INTEGER NOT NULL,   -- precomputed source_rank()
    publication_id TEXT NOT NULL,
    published_at  INTEGER NOT NULL,
    subtype       TEXT,
    group_key     TEXT          -- assigned during grouping; see below
)
plan_ojs_node(key INTEGER PRIMARY KEY, label INTEGER NOT NULL)  -- legacy nodes
plan_ojs_edge(a INTEGER NOT NULL, b INTEGER NOT NULL)           -- symmetric
```

Encode an OJS key `(year, number)` as `year*10_000_000 + number` (number < 10^7;
year ≤ 2100 → < 2.1e10, fits i64). `MIN(encoded)` = the earliest `(year,number)` =
today's `root_minimums` representative, exactly.

### Phase 1 (stream — unchanged shape, writes to disk instead of a Vec)

Per notice, exactly as today compute the identity via `Ident::read` (single source
of truth for the era-specific field logic), but INSERT it into `plan_notice`
instead of pushing to a Vec. For legacy notices also INSERT `ojs_self` and each
edge target into `plan_ojs_node` (label = self) and both directions into
`plan_ojs_edge`. Resolve mentions as today. Peak RAM here = one read chunk +
org_of; the plan is on disk.

### Grouping (SQL on disk — O(1) RAM)

1. Keyed + island in one UPDATE:
   `group_key = CASE WHEN procedure_key IS NOT NULL THEN procedure_key
                     WHEN NOT (legacy AND ojs_self IS NOT NULL) THEN 'island:'||notice_id
                     ELSE NULL END`
   (keyed → the raw BT-04/uuid, which is what `tenders.procedure_key` stores;
   island → a unique non-colliding handle; legacy left NULL for step 3.)
2. Legacy label propagation: loop in Rust —
   `UPDATE plan_ojs_node SET label = (SELECT MIN(v) FROM (
        SELECT plan_ojs_node.label AS v UNION ALL
        SELECT n2.label FROM plan_ojs_edge e JOIN plan_ojs_node n2 ON n2.key=e.b
        WHERE e.a = plan_ojs_node.key))`
   until `SUM(label)` is stable (monotone-decreasing → converges in ≈ component
   diameter; real OJS chains are short; guard with a max-iteration cap).
3. Assign legacy group_key:
   `UPDATE plan_notice SET group_key =
      'ojs:' || (n.label/10000000) || '-' || printf('%06d', n.label%10000000)
    FROM plan_ojs_node n WHERE n.key = plan_notice.ojs_self AND group_key IS NULL`
   (verify turso `UPDATE … FROM`; fallback = correlated subquery form).
4. Create the Phase-2 index UPFRONT-or-here (index build is ~31 s/1M rows in
   turso — budget for it): `plan_notice(group_key, published_at, source_rank,
   publication_id, notice_id)` — the fold order, so Phase 2's scan needs no sort.

### Phase 2 (stream whole-Tender batches from the plan — O(batch) RAM)

Scan `plan_notice` ordered by the index above. Rows arrive grouped by `group_key`,
each group already in fold order. Accumulate whole groups into a batch until the
notice budget is hit (never split a group — the ordering guarantees a group is
contiguous). Per batch: `parsed_by_ids` + `mentions_by_ids` (as today), build +
bind states, and per group derive the projection —
`procedure_key = group_key unless it starts 'island:' → NULL`;
`island_notice_id`, `kind = kind_of(first.subtype)`,
`source = 'ted' if any member is ted else first member` — then `fold` + apply.
Peak RAM = one batch's states, independent of corpus.

### Residual: `org_of`

The Organization dedup map stays in RAM (issue 19 made it O(1)-per-mention by
preloading, replacing an O(n²) per-mention scan). It is O(distinct identified
orgs) ≈ 150–250 MB at 7.5M — sub-linear and within the ~1 GB target once the plan
is off-heap. Making it disk-backed means per-mention indexed lookups against
`UNIQUE(country,identifier_kind,identifier)`, which risks re-introducing the
issue-19 slowdown, so it should only be done if a benchmark shows it is needed.
Recommendation: keep in RAM for now; note it as the next residual if strict
corpus-independence is later required.

## Operability requirement: progress logging in BOTH phases (team-lead, 2026-07-24)

A full-corpus projection runs many minutes; Phase 1 streaming the plan was SILENT
for 20-30 min, so from the outside it looked dead — that silence made the issue-57
incident far harder to diagnose (Lennart flagged "does not show progress"). Not
gold-plating; a real operability gap. Required:

- Phase 1: a heartbeat on a cadence (every `PLAN_HEARTBEAT` = 500k notices, or
  ~30s) — `[project] phase 1: <planned>/<total> notices planned` (total from a
  cheap `COUNT` up front).
- Phase 2: a per-batch heartbeat with a running fraction —
  `[project] phase 2: <M>/<total> tenders applied`.
- Clear phase-transition lines (plan / group / apply, with elapsed) and a final
  completion summary (`[project] done: N notices → T tenders … in Xs`).
- A smoke test that the heartbeat fires in both phases.

Implemented via a `Progress` event + `project_with_progress` (mirrors
`process.rs`'s `on_progress` hook); `project` logs to stderr, and the smoke test
`the_projection_reports_progress_in_both_phases` observes the events directly.

## Reproduce first

Extend `tests/project_memory.rs` to assert peak RSS is ~FLAT across two corpus
sizes at the DEFAULT (production) batch — e.g. 6k vs 12k peak within ~1.2×, not
the current 1.27×-that-still-creeps — proving corpus-independence, stronger than
the per-batch bound. Keep the whole-RAM A/B for contrast.

## Preserve equivalence

`tests/project_equivalence.rs` must stay byte-identical: the disk-backed grouping
(keyed chains, legacy transitive merges + ADR-0003 absorbed-key retirement,
islands, cross-time attach) must produce the exact same canonical layer. The
existing 22 projection tests are the correctness oracle.

## Sign-off decisions (team-lead, 2026-07-24)

- Scratch tables in the MAIN db: approved. Two non-negotiable WAL caveats (we
  just fixed the WAL, issue 42 — must not regress):
  - (a) The ~7.5M plan-row inserts are a WAL burst — CHECKPOINT (TRUNCATE)
    periodically DURING the Phase-1 plan build (same cadence as Phase 2), and
    prove the WAL stays bounded through the plan build in the test.
  - (b) Clear the plan tables at BOTH start AND end of the run — don't leave
    transient rows in the durable DB between projections (they'd bloat the
    off-box snapshot, issue 23). Confirm no path ships a snapshot with a
    half-built plan (daily snapshot runs after project → normally fine).
- `org_of` stays in RAM: accepted as the known residual. Do NOT reintroduce
  per-mention disk lookups (issue-19 O(n²) regression). Revisit trigger: a
  benchmark, only if distinct-orgs ever grows material against the <4 GB bar.

## Tradeoffs

- Extra disk I/O + WAL churn: Phase 1 now writes ~7.5M plan rows + edges; Phase 2
  re-reads the plan. All bounded-memory; acceptable for a rebuild that is already
  minutes-to-hours. `parsed` is still read twice (unchanged from issue 57).
- Index build cost (~31 s/1M rows) — a one-time per-run cost; budget for it.
- Label propagation iterations ~ chain diameter (small in practice); capped.

## Coordination with issue 58

Both are "don't hold the corpus." This (59) = corpus-independent MEMORY for the
full rebuild path, via the disk-backed plan. 58 = incremental TIME for the daily
path (only re-project touched Tenders). Recommendation: keep them separate — 59
builds the disk-backed plan substrate; 58 then scopes Phase-1 inserts to touched
notices + touched-key expansion, reusing 59's plan tables and grouping SQL. 59 is
independently deployable value (kills the latent memory wall); 58 builds on it.

## Acceptance

- Peak RSS flat across corpus size at the production batch (extended
  project_memory), comfortably < 4 GB (target ~1 GB).
- Canonical output byte-identical to the whole-RAM projection (project_equivalence
  + 22 existing tests).
- rebuild:true full re-projection completes within RAM at 7.5M+ scale.

## Comments

2026-07-24 (proj-fix): Design drafted + the union-find-on-disk crux spiked green
in turso (label propagation, no WITH RECURSIVE). Awaiting team-lead sign-off on
the approach (esp. scratch-tables-in-main-DB vs a separate scratch file, and the
org_of residual decision) before reproducing + implementing. Do NOT deploy.
