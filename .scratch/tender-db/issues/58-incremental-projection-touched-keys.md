# 58 — Incremental projection: re-project only the Tenders touched since last run

Status: in-implementation (2026-07-24, proj-fix; team-lead approved all 3 sign-offs) — v1 keyed/island + legacy fallback landed on branch issue62-defer-org-indexes, green; timing curve + supervisor wiring remaining
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

## Implementation status (2026-07-24, proj-fix)

All three sign-offs APPROVED by team-lead (boolean watermark; v1 legacy fallback +
defer v2; sequence 60/62-rebuild + 58-daily). Landed on branch
issue62-defer-org-indexes (reproduce-first, NOT deployed):
- df0c2b6 — slice 1: `notices.projected` watermark (column + partial index in
  migrate; cleared on every transition to 'parsed' via set_parse_state; set by
  Phase 2; reset by rebuild). Migration backfills projected=1 for already-folded
  notices so the first incremental after upgrade isn't a whole-corpus scan.
- bf2e632 — slice 2: `project::project_incremental`; store helpers
  `touched_existing_tender_ids` / `notice_ids_for_tenders` /
  `retire_regrouped_tenders`. Keyed late-attach proven byte-identical to full.
- 64bcc27 — slices 3-4: mixed-delta untouched-invariance + legacy full-rebuild
  fallback (loud log) tests, both byte-identical to full.
- (this commit) slice 5: reproduce-first timing curve.

NOTE on island-upgrade (fixture case b): retiring an EXISTING island Tender when
its notice gains a key requires the notice's parsed layer to change IN PLACE. The
current pipeline is append-only by (source, publication_id, content_hash) — a
content change mints a NEW notice id (a separate keyed Tender alongside the old
island; both a full and incremental run keep both, and incremental matches full),
and there is no in-place re-parse path today. So `retire_regrouped_tenders` is a
correct defensive generalization that fires via the legacy fallback and would fire
for a future in-place re-parse path, but cannot be produced through the normal
ingest path now. Retirement on merge is covered for legacy by the existing
`a_late_edge_merges_two_legacy_tenders` (full path) + the fallback.

SUPERVISOR WIRED: `Spec::Project { rebuild:false }` (the daily path) now calls
`project_incremental`; `rebuild:true` (initial/periodic rebuild) still does the
full bounded-streaming projection. Test `daily_project_job_is_incremental_while_
rebuild_is_full` asserts the daily summary reports only the delta ("1 notices")
while a rebuild reports the whole corpus. The branch is now a coherent deployable
unit (62 + 58 + wiring).

FUTURE LINKAGE (island-upgrade → `retire_regrouped_tenders` goes live): the day an
in-place RE-PARSE path is built (quarantine reprocess / issue 41 INTERNAL_OJS
reprocess / profile-fix reprocess), a notice's parsed layer changes on its
existing id, `set_parse_state` clears its `projected` (the watermark already
anticipates this), the incremental run re-groups it, and `retire_regrouped_tenders`
retires its now-empty old Tender. THAT is the trigger to add an island-upgrade
output-identity test (dormant/defensive until then).

DEPLOY (team-lead sequencing): the 58+62 batch deploys AFTER the current prod run
completes + the built layer is verified per CONTEXT.md, so the next daily
projection is incremental (seconds) instead of another ~5h full scan.

## Comments

2026-07-24 (proj-fix): Original capture had the correctness trap + design
direction. Deepened after mapping the apply/reconcile path (already
incremental-safe — Phase 2 unchanged), the append-only cursor (coherent by
construction), and the reprocessing model (append-only by content_hash; re-parse
flips parse_state in place → watermark must be a per-notice marker, not an id).
The legacy transitive-reach is the only genuinely hard piece and it is
side-stepped for daily by the rebuild fallback (legacy is historical/backfill).

## v2 DESIGN — durable OJS adjacency, so a legacy delta folds its own components (2026-08-17, owner)

The v1 fallback (any legacy notice in the delta → full projection) is issue 179's remaining half:
the write side is fixed (scoped stale-stamp, `19c7590`), but planning still pays the whole corpus.
This design removes that by making the legacy edge graph QUERYABLE from persisted state.

### Ground truth (verified in code)

`Ident::read` already computes, per legacy notice: `ojs_self: Option<OjsKey>` (its own OJS number,
from `publication_id` or `LEGACY_OWN_NUMBER_FIELDS`) and `ojs_edges: Vec<OjsKey>` (every
`is_ref` OJS-scheme id — the REF_OJS chain edges), both encodable as one i64 (`encode_ojs`,
year×1e9+number). Grouping unions notices sharing any key. The inverse mapping (key → the notices
carrying it) is what an incremental closure needs and what nothing persists: `notices.publication_id`
is NOT usable as that inverse (one key ↔ many era spellings; `ojs_key` normalizes many-to-one).

### The durable store

New table, legacy-only, written where `Ident` is already in hand:

    legacy_ojs_keys (ojs_key INTEGER NOT NULL, notice_id INTEGER NOT NULL,
                     PRIMARY KEY (ojs_key, notice_id))   -- self and edge rows alike

- Self vs edge needs no flag: the closure treats them identically (a shared key groups, whichever
  side it came from).
- Writers: (1) the full/rebuild plan build (visits every notice; rewrite-all idempotent),
  (2) the incremental pass-1 for new legacy notices (BEFORE the closure walk, so the delta's own
  rows are queryable), (3) a one-time backfill job for the standing corpus (batched sweep of the
  parse layer's `notice_ids` scheme='ojs' rows + publication ids — the issue-42 checkpoint pattern).
- Completeness witness: `legacy_adjacency_watermark` (max notice id covered). The incremental path
  uses the closure ONLY when the watermark covers the corpus; otherwise it falls back to full
  exactly as today. Self-healing forward: pass-1 writes rows before raising the watermark.
- Size: ~6-7M legacy notices × ~1-3 keys ≈ 15M rows + PK — a small fraction of one value table.

### The closure walk (replaces the v1 fallback when the witness holds)

    seed_keys   = ∪ (self ∪ edges) of the delta's legacy notices     (from pass-1's Idents)
    loop until no new keys:
        notices  = SELECT notice_id FROM legacy_ojs_keys WHERE ojs_key IN (frontier)
        tenders  = caused_by lookup over those notices               (tender_versions_notice index)
        notices += notice_ids_for_tenders(tenders)                   (existing fn)
        keys     = SELECT ojs_key FROM legacy_ojs_keys WHERE notice_id IN (new notices)
    all_ids = closure notices ∪ delta; plan scoped; SAME grouping SQL; retire_regrouped over the
    touched tenders (absorption stays inside the closure by construction).

Safety bound: if the closure exceeds a cap (500k notices, say), log and take the full path — a
pathological component (issue 68's class) must degrade to today's behavior, never to a wrong scope.

### Red tests, in order

1. **Bridge merge** (the correctness crux): two existing single-notice legacy tenders A and B; a
   delta notice referencing both their keys → ONE merged tender, the loser retired/absorbed, all
   three notices in the chain — asserted equal to what a full projection of the same corpus yields.
2. **Late back-reference**: existing notice references key K; the delta notice IS K (its self) —
   the closure must find the existing notice through the key row written when IT was planned.
3. **Watermark gate**: delta with the witness stale → full fallback (today's behavior), loudly.
4. **Cap**: synthetic over-cap component → full fallback, loudly.
5. **Fold-source invariance**: the scoped legacy plan through both Phase-2 folds, byte-identical
   (the existing invariance-test shape, extended to a legacy corpus).

### Rollout order

1. Table + writers behind the watermark (no behavior change; fallback still fires) + backfill job.
2. Run the backfill on prod (quiet window; batched); watermark raised.
3. The closure path replacing the fallback, cap-guarded; deploy; verify a small real legacy
   reclaim folds scoped (journal shows closure size, not "re-projecting the whole corpus").
4. Only then: the next era refold measures the whole 179 win end to end (scoped stamp + scoped plan).

Estimated: 2-3 firings. Steps are independently shippable; each lands green on its own.

### v2 progress

- **Step 1 SHIPPED** (2026-08-17, `6c97734`, deployed 06:09 UTC): `legacy_ojs_keys` +
  choke-point writer inside `insert_plan_tx` (self ∪ edges, legacy-gated, INSERT OR IGNORE) +
  `legacy_adjacency.watermark` with the establish/advance lifecycle (full build establishes,
  incremental advances, advance refuses on a never-established base). Lifecycle test green.
- **Step 2 DEPLOYED & RUNNING** (2026-08-17): `backfill-legacy-adjacency` admin job —
  `legacy_parsed_chunk` bounded read (SQL mirror of `is_legacy_profile` as pre-filter only;
  `Ident::read`'s verdict decides writes), org-names job shape, drift test proves the sweep
  re-derives the choke point's rows set-identically. Deployed rev `89070e1` 08:34 UTC, backfill
  enqueued 08:39 UTC (job id 1). PENDING VERIFY on completion: `legacy_adjacency.watermark`
  = MAX(id) of parsed notices, spot-check a known 2008 chain's keys.
- Step 3 (the closure walk behind the watermark gate) not started.
