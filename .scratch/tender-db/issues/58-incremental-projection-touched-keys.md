# 58 — Incremental projection: re-project only the Tenders touched since last run

Status: design-ready (deepened 2026-07-24 by proj-fix; awaiting team-lead sign-off on scope/phasing)
Severity: MEDIUM (daily-wall-clock optimization, not an outage) — but it is the
BIGGEST projection lever: every projection today re-reads+re-plans the whole
12.4M-notice / 254GB corpus (~5h15m measured), which makes the daily job
untenable and makes 60/62 (rebuild-time read amplification) largely moot for
daily runs (a small delta = no thrash, no full scan).
Blocked by: 59 must be DEPLOYED and validated in prod first — sits on 59's
disk-backed plan substrate. 59 is now validated by the live run, so this is
UNBLOCKED for design; implementation waits on the team-lead's go.

## Problem

`project()` reads and re-folds the WHOLE corpus on EVERY run. In non-rebuild
mode Phase 1 still loops `parsed_chunk(after_id, …)` from `after_id = 0`
(project.rs:354) over `SELECT … FROM notices WHERE parse_state='parsed' AND id>?`
(canonical.rs:762) — ALL parsed notices — and Phase 2 applies every group. The
`rebuild` flag only controls whether `clear_canonical()` runs first. "Project
only what's new" is achieved today PURELY by `apply_tender_tx`'s idempotent
early-return (zero writes for an unchanged Tender), not by scanning fewer
notices. So a daily run does O(corpus) read+fold work to absorb a few thousand
new notices, and that cost grows with the corpus forever.

## Goal

Daily projection cost scales with **notices changed since the last projection**,
not corpus size. Re-project each touched Tender IN FULL (re-deriving its whole
version chain). The `rebuild:true` full re-projection keeps the issue-57/59
bounded-streaming path and must always fit in RAM.

## KEY ENABLING FACT (verified 2026-07-24) — the apply path is already incremental-safe

`apply_tenders` / `apply_tender_tx` (canonical.rs:1539/1586) is a per-Tender,
natural-key upsert-replace, NOT a global rewrite:
- Identity looked up and REUSED by `procedure_key` / `(source, island_notice_id)`
  (UNIQUE constraints, canonical.rs:67-68) — surrogate ids stable across runs.
- The version chain is diffed by `caused_by_notice_id` prefix; only the changed
  suffix is deleted+rewritten; an identical chain hits an idempotent EARLY RETURN
  (zero deletes, zero writes, zero change rows).
- Every delete/write is scoped to one `tender_id` — there are NO global deletes
  in the reconcile path (the only unscoped `DELETE FROM` is in `clear_canonical`,
  the rebuild path). Satellite/lot/result ids are reused via natural-key lookups.

Consequence: **feeding Phase 2 only the touched Tenders leaves every untouched
Tender byte-for-byte unchanged, and produces the touched ones identical to a full
NON-REBUILD projection.** So Phase 2 needs ZERO changes. The change-feed cursor
(canonical.rs:361) is AUTOINCREMENT, append-only, never renumbered, and emission
is content-diff-gated (no spurious events on re-apply). An incremental run simply
appends the change rows for what it touched — coherent by construction (issue 46
preserved).

Output-identity BASELINE, therefore, is the current **full non-rebuild**
projection (which reuses ids), NOT `project --rebuild` (which clears +
AUTOINCREMENT-renumbers all surrogate ids and mints a fresh cursor sequence).

## Correctness trap (load-bearing — why this is subtle)

A new notice can attach to an OLD Tender (a correction/award/result referencing an
earlier notice; ADR-0001). "Incremental" therefore means: find the grouping keys
TOUCHED by the changed notices, then re-project each touched Tender IN FULL
(loading its pre-existing notices too). The touched-set is NOT just the changed
notices' own new keys — it is the union of:

1. **New identity** of each changed notice (its current parsed grouping key).
2. **Old membership** of each changed notice (the key/island of the Tender it
   currently belongs to, via `tender_versions.caused_by_notice_id → tenders`) —
   so a REGROUPED notice re-derives (or retires) its former Tender too.
3. For **legacy**, the transitive union-find COMPONENT(s) its OJS edges reach —
   a new legacy notice can BRIDGE two previously-separate components into one
   (the ADR-0003 late-edge merge; `a_late_edge_merges_two_legacy_tenders`), and
   the merge retires the absorbed key with `removed` events.

Getting this wrong yields a split or stale Tender a full rebuild would have
merged/updated — silent, not a crash.

## Design

### 1. Watermark = a per-notice "projected" marker (not an id cursor)

An `id > watermark` cursor is INSUFFICIENT: notices are append-only by
`(source, publication_id, content_hash)` (a content revision mints a NEW high id
— caught by id), BUT a re-PARSE of an existing low id (quarantine reprocessing,
a profile fix, delayed processing) flips `parse_state` in place WITHOUT changing
the id — an id-only watermark misses it.

Proposal: add `projected INTEGER NOT NULL DEFAULT 0` to `notices` (+ partial
index `WHERE parse_state='parsed' AND projected=0`). Contract:
- The processor sets `projected=0` whenever it (re)writes a notice's parsed layer
  (new parse, quarantine→parsed reprocess, re-parse). New rows default to 0.
- Phase 2 sets `projected=1` for every notice it applies (in the same batch txn).
- `rebuild:true` resets all rows to 0 (part of `clear_canonical`).
- The incremental CHANGE-SET S = `notices WHERE parse_state='parsed' AND projected=0`
  — bounded by the daily delta, and id-independent so it catches re-parsed olds.

(Alternative considered: stored max-projected-id + a `parsed_at` timestamp column
to catch re-parses. Rejected — two pieces of state and a time-window join vs one
boolean with an exact set. The boolean is simpler and precise.)

### 2. Touched-set discovery → scoped plan seed

Reuse the EXISTING plan machinery unchanged: `build_plan_groups`,
`next_plan_batch`, `plan_counts`, `plan_legacy_keys` all operate over whatever is
in `plan_notice`. Incremental only changes WHAT gets loaded into `plan_notice`:

a. **Change-set S** (above).
b. **Touched keys K** = for each notice in S: its new-identity key ∪ its
   old-membership key (join S to `tender_versions.caused_by_notice_id → tenders`).
   - keyed → `procedure_key`; island → `island:<notice_id>`.
c. **Load the FULL notice set of every key in K** into `plan_notice`: the
   changed notices plus, for each touched keyed/island Tender, its pre-existing
   notices (`SELECT caused_by_notice_id FROM tender_versions WHERE tender_id=?`).
   This is bounded by |touched Tenders| × their chain lengths, not the corpus.
d. Run `build_plan_groups` over this scoped `plan_notice`, then Phase 2
   (`next_plan_batch` → `apply_plan_batch`) over it. Untouched Tenders are never
   read or written.

### 3. Legacy transitive reach — the one hard part; RECOMMENDED PHASING

The legacy component walk needs the EXISTING OJS edge graph, but edges live only
in the transient `plan_ojs_edge` (cleared per run). Two ways:

- **(Recommended v1) Full-rebuild fallback when the delta contains legacy
  notices.** In steady-state DAILY operation the delta is eForms + DÖE
  (keyed/island) — TED legacy is the pre-2024 historical era, loaded by BULK
  BACKFILL (which uses `rebuild:true` anyway), never in the near-real-time feed.
  So: if S contains any `legacy=1` notice, fall back to a full projection for that
  run (correct, simple, and effectively never triggers daily). Ship incremental
  for keyed/island first — that covers ~all daily volume.
- **(v2, deferrable) Durable legacy OJS adjacency index.** Persist a bounded
  `legacy_ojs_edge(a,b)` (+ `notice→ojs_self`) index — legacy notices are a
  frozen historical subset — so a legacy delta can walk only its reachable
  component(s), union existing components, re-run label propagation on that
  subgraph, and merge/retire. Only worth building if legacy notices ever arrive
  incrementally (e.g. a new legacy Source or late corrections).

### 4. Scoped retirement

`retire_absorbed_legacy_tenders` (canonical.rs:2125) currently compares ALL
`ojs:%` tenders against the produced set — in incremental mode `produced` is only
the touched keys, so as-is it would wrongly retire every untouched legacy Tender.
Fix: pass BOTH the touched INPUT legacy keys and the produced OUTPUT keys; retire
only an `ojs:%` tender whose key was a touched INPUT but is absent from the
OUTPUT (i.e. it merged away). With the v1 legacy fallback this path only runs on a
full rebuild, so no change is needed until v2.

## Reproduce first (timing curve — the time analogue of project_memory)

Before implementing: prove the CURRENT full-rescan projection's wall-clock scales
with corpus while incremental scales with delta.
- Build existing corpora at sizes {100k, 300k, 1M} notices, project each once.
- Apply a FIXED small delta (e.g. 1k new keyed notices, a fraction attaching to
  existing Tenders) and time (a) the current non-rebuild full projection vs (b)
  the incremental run, at each corpus size.
- Expect (a) to rise ~linearly with corpus; (b) to stay ~flat (seconds), scaling
  with the delta. That curve is the acceptance evidence.

## Acceptance

- Daily projection wall-clock scales with notices-changed, not corpus (curve above).
- OUTPUT-IDENTITY vs a full NON-REBUILD projection (reused ids) — proven on a
  fixture covering: (a) a late notice attaching to an old keyed Tender across
  time, (b) an island-upgrade where a re-parse gives a formerly-island notice a
  shared BT-04 (old island retired, joins the keyed Tender), (c) [v2] a legacy
  bridge merging two existing components. Digest-compare like
  `project_equivalence::snapshot`.
- `rebuild:true` still works within RAM via the issue-57/59 bounded path.
- Change-feed/cursor coherence preserved (issue 46) — guaranteed by the
  append-only diff-gated cursor; assert the incremental run emits exactly the
  change rows for the touched entities and none for untouched.

## Open sign-off points for team-lead

1. Watermark: OK to add `notices.projected` + the processor "clear on (re)parse"
   contract? (vs a max-id+parsed_at scheme.)
2. Legacy phasing: ship v1 (keyed/island incremental + full-rebuild fallback on
   any legacy delta) first, defer v2 durable adjacency? (My recommendation: yes.)
3. Priority vs the 60/62 batch: incremental makes 60/62 moot for DAILY runs but
   NOT for the periodic full `rebuild:true`. Sequence: deploy 60/62 for the next
   rebuild, build 58 for daily?

## Comments

2026-07-24 (proj-fix): Original capture had the correctness trap + design
direction. Deepened after mapping the apply/reconcile path (already
incremental-safe — Phase 2 unchanged), the append-only cursor (coherent by
construction), and the reprocessing model (append-only by content_hash; re-parse
flips parse_state in place → watermark must be a per-notice marker, not an id).
The legacy transitive-reach is the only genuinely hard piece and it is
side-stepped for daily by the rebuild fallback (legacy is historical/backfill).
