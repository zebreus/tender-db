# 111 — the deferred indexes have no guaranteed builder: nothing detects or repairs a missing one

Status: open — **LOW / latent risk. NOT an active incident.** Downgraded 2026-08-03 after
the motivating hypothesis was REFUTED empirically (see "Refutation" below). The general
class is real and worth closing eventually; there is no live defect behind it. Code work
STOPPED.
Kind: correctness / performance (latent — silent under-indexing of the live layer)
Blocked by: —
Relates to: 112 (the independent gate — *strengthened* by the refutation), 89, 82, 62/60, 83, 64

## Refutation of the motivating incident (2026-08-03, run-driver)

**The index existed the whole time.** run-driver read the schema off the post-refold
snapshot: `tender_version_bid_parties_version` is PRESENT. **Job 534's `reindex` built
it** — the operator step issue 89 called for was actually taken. run-driver also EQP'd
the `tender_detail` satellite reads: no full scan among them.

So the `/v1/tenders/{id}` ~2.2s regression was **not** a missing deferred index, and the
real cause is query-side (`lots_of` reusing the paginated list query — see the separate
lots issue). Everything below about *mechanism* is still accurate as a description of
the code; what was wrong was the claim that it had *happened*.

### Why two careful code-reads both got it wrong

proj-fix and sdk-vendor converged independently, and the enumeration proof
("`DROP INDEX` appears exactly once in the non-test tree…") was **correct about the
code**. It was answering the wrong question. The live DB's index set is a product of
*which jobs actually ran*, not of what the code paths would do — and job 534 was a
historical fact no amount of source reading could reveal. We inferred live state from
code plus a deploy timeline, and never read the live schema, which is the one thing that
could have settled it in seconds.

This is the verification-input-discipline lesson landing on its authors: *a check is only
as trustworthy as its knowledge of its own inputs.* Our input was the repo; our
conclusion was about the database.

Second-order lesson, worth more than the first: the hypothesis grew *more* persuasive as
it explained more (89 → the 4-index class → issue 95's 5h07m → "why folds run longer
than estimated"). Explanatory reach felt like corroboration when it was actually
compounding risk — every added consequence rested on the same unverified premise. The
5.2h ≈ 5h07m arithmetic that felt like a keystone was a coincidence built on a
back-derived parameter. **A story that keeps getting better is a reason to go check the
premise, not a reason to believe it harder.**

## What is still true, and why this stays open at LOW

The structural gap is genuine and unaffected by the refutation: a **newly-added**
`DEFERRED_TENDER_INDEXES` entry still has no builder until a `rebuild=true` completes or
someone fires `reindex`. Issue 89 needed exactly that manual step, and it happened only
because a human remembered. Four indexes remain in the never-auto-materialized class
(the other seven are re-created by the open-time schema batch). The fix below is the
right one *if and when* this class is worth closing — but there is no incident driving
it, so it waits behind work that has one.

## The question this answers

*Does the full-refold path rebuild the deferred tender indexes after it rewrites the
tables?* — **Yes on `rebuild=true`, and the incremental path never drops them.** The
fold is not the bug. The bug is that "deferred" means *no automatic path builds it at
all*, and nothing anywhere reports that it is missing.

## The two index classes

`DEFERRED_TENDER_INDEXES` (canonical.rs:1376) + the org pair are built by
`build_tender_indexes` / `build_organization_indexes`. Seven of those twelve are ALSO
emitted by the open-time schema batch (`canonical::SCHEMA`) or `migrate()`, so
`Db::open` re-creates them if a rebuild dropped them — expensively, but automatically.

**Four are not, by deliberate design** (a schema-batch `CREATE INDEX` would build over
the full prod table at every `Db::open` — the multi-hour/hanging boot of issues 82/83/62):

| index | on | serves | cost when missing |
|---|---|---|---|
| `tender_version_bid_parties_version` | `(tender_id, seq)` | `tender_detail` (read.rs:794) | full scan per `/v1/tenders/{id}` — **issue 89** |
| `tenders_procedure_key` | `tenders(procedure_key)` | the incremental identity probe | full scan of 8.1M `tenders` **per folded Tender** |
| `tenders_island` | `tenders(source, island_notice_id)` | the incremental identity probe | same |
| `organizations_identity` | `organizations(country, kind, identifier)` | the Phase-1 mention resolver's probe | full scan of ~30M `organizations` per new mention |

For those four, the index exists on a live DB **only if** one of two things happened:

1. a `rebuild=true` (or salvage/resume) projection ran **to completion** on a binary
   that already listed it — `project.rs:665-666`, gated `if rebuild`; or
2. an operator manually fired the `reindex` job (`Spec::Reindex`, supervisor.rs:762).

Nothing else. Not boot, not `/health`, not the daily fold, not the verify suite.

## Why the `/v1/tenders/{id}` ~2.2s regression is exactly this

`a572544` (issue 89) added `tender_version_bid_parties_version` as the tenth deferred
entry. Issue 89's own Status line says it plainly: *"awaiting deploy + reindex"*. A
deploy alone cannot create it — that is the whole point of deferring it. So on prod it
exists only if a `rebuild=true` completed after `33dfba7` went out, or someone ran
`reindex`. The DE-1.x re-fold went through `Refold` + the incremental fold
(`rebuild=false` → `Phase2::Buckets`), which:

- never calls `strip_tender_indexes` (so it cannot have dropped it), and
- never calls `build_tender_indexes` (so it cannot have built it).

The index was therefore never built, not dropped. Issue 89 was fixed in code and never
fixed on prod, and nothing said so for two days.

### Diagnostic that discriminates the two failure stories

```sql
SELECT name FROM sqlite_master WHERE type='index' AND name IN
 ('tender_version_bid_parties_version','tenders_procedure_key','tenders_island','organizations_identity');
```

- **only `tender_version_bid_parties_version` missing** → the last completed
  `rebuild=true` predates `a572544`; the missing post-deploy `reindex` is the whole story.
- **more than one missing** → the last `rebuild=true` never finished its end-of-fold
  index build (the issue-83 ENOSPC shape), and the *incremental fold* has been
  full-scanning `tenders`/`organizations` on every daily tick. That is far worse than
  the detail endpoint and would be invisible except as "the daily fold got slow".

## Root cause

Deferring an index is a correct performance decision (issues 60/62/82/83) that silently
transfers a *durability obligation* to an operator's memory. The obligation is
undischargeable in principle: a code change that adds a deferred index can never take
effect by deploying, and the system has no way to notice.

Three separate incidents are the same design gap: 82 (`tenders_current_published`
dropped by a rebuild, rebuilt only at next boot — a 49min boot), 89 (a new deferred
index that a deploy cannot create), and this one (89 re-emerging because the reindex
step was never encoded anywhere the system enforces).

## Proof that no other path touches these indexes

The claim "nothing else builds or drops them" is worth proving exhaustively rather than
by inspecting the paths one happens to think of. Enumerating **every** index-DDL site in
the non-test source settles it:

- **`DROP INDEX` appears exactly ONCE in the entire non-test tree** — canonical.rs:1548,
  the body of `strip_tender_indexes()`. Its only non-test caller is project.rs:565,
  inside `if rebuild`. There is no other statement anywhere that can drop a deferred
  tender index. (`strip_organization_indexes` drops by DROP TABLE instead;
  project.rs:544, inside `if rebuild && !resume`.)
- **The only `CREATE INDEX` sites that emit the four** are canonical.rs:1558 (the
  `build_tender_indexes` loop) and canonical.rs:1348 (`organizations_identity`).
  Callers: project.rs:665-666 (`if rebuild`) and supervisor.rs:767-768
  (`Spec::Reindex`). Nothing else — not `migrate()`, not any SCHEMA batch, not retire,
  not snapshot/restore.
- **`Spec::Refold` executes no projection and no DDL at all** (supervisor.rs:772-795):
  it counts, guards on `expect`, calls `unmark_projected_for_profiles`, and returns.
  The fold is a *separate* trailing `project rebuild=false` job →
  `project_incremental` → likewise no index DDL on any branch.

So a `rebuild=false` run cannot drop an index, and cannot build one. This is stronger
than reasoning about whether row rewrites preserve an index (they do — SQLite maintains
indexes incrementally): the rebuild=false path never executes index DDL at all, so the
question does not arise.

Corollary: **if `tender_version_bid_parties_version` is absent on the live DB now, it
was absent before the re-fold, and has been absent since `a572544` was deployed.**

## Fix — a deploy/migration-path gap, NOT a fold-path one

The fold path is already correct for `rebuild=true`. What is missing is any detection
that a deployed build's `DEFERRED_TENDER_INDEXES` const contains entries **not yet
materialized on the live DB** — an unapplied migration with no migration tracking.

**Detect loudly at boot, report, AND auto-enqueue a guarded repair.** Building
*synchronously at boot* is issue 82/83 and must not be reintroduced in any form — but a
durable background job is not that, and "report vs repair" is a false dichotomy: the
detector should do both.

1. **`Db::missing_deferred_indexes() -> Vec<&'static str>`** (store/canonical.rs). One
   `SELECT name FROM sqlite_master WHERE type='index' AND name IN (…)` over the const's
   names plus the org pair, returning the absent ones. O(1) — a metadata-only lookup.
   The *check* was never the expensive part; only the build was. This is the piece that
   turns the const into a declared-vs-actual schema comparison.
2. **Detect at `Supervisor::init` (after `recover()`) and report.** Log the missing
   names loudly, and surface them as a standing state: `/health` **degraded** + an admin
   banner naming the missing indexes and the remedy. Degraded rather than failing: the
   service is serving, just under-indexed. Reporting is kept in full even though the
   repair is now automatic — the operator must still be able to see *why* a reindex is
   running and that the layer was under-indexed until it finishes.
3. **The degraded signal must reach a decision-maker, not just a banner.** Report-only
   discharges the obligation *only if the report is seen* — otherwise "the operator's
   memory" is merely replaced by "the operator opens the dashboard", which is still a
   human-in-the-loop assumption this project does not hold (it runs mostly
   autonomously). So the standing alerting routine — the local snapshot-ring + Claude
   routine that stands in for external monitoring here (there are deliberately no
   external monitors) — **must treat health-degraded-for-missing-indexes as an
   actionable alert**. Without this line the detector is a tree falling in an empty
   forest, and the failure mode reverts to exactly the one this issue exists to close:
   a known-required step that nothing makes anyone do.
4. **AND auto-enqueue `Spec::Reindex`** on detection. The durable queue is a background
   worker: boot never blocks, so 82/83 stay fixed. The op is already idempotent
   (`CREATE INDEX IF NOT EXISTS` loops), so it builds only what is missing and one run
   covers all four. This is what makes the fix self-*applying* rather than merely
   self-announcing, which the autonomous posture requires.
5. **Dedup**: do not stack a reindex if one is already queued or running. Repeated
   restarts during the broken window must not pile up jobs.
6. **Pre-flight free-space guard inside `Spec::Reindex`** — the change that makes
   auto-repair safe. Before starting, check free space on the DB/TMPDIR volume against a
   conservative threshold and **REFUSE-and-alert loudly and distinctly** rather than
   beginning a build it cannot finish. Deliberately a crude threshold, not a per-index
   size estimate. This converts issue 83's failure mode from *"silent burnt night, ENOSPC
   mid-build, layer still unindexed"* into *"loud refusal before starting"*, and it
   benefits the operator-fired path identically. The guard belongs in the op — the place
   that knows it is about to spend hours and disk — not in the caller that decides to
   queue it.

Concurrency and the escape hatch: jobs run **sequentially**, so an auto-enqueued reindex
cannot run concurrently with a fold — that is the mitigation for "a build at a moment
nobody chose". An operator who wants to defer a queued one retains `TENDER_DROP_JOBS`.

**Reconsidered and ADOPTED: auto-enqueuing `Spec::Reindex`** — superseding the earlier
*considered-and-rejected* note recorded in this issue, on new evidence.

The rejection priced the cost of a missing index as "one degraded endpoint", against the
risk of an unattended multi-hour build dying on ENOSPC. The structural finding below
changed both sides of that trade:

- `touched_existing_tender_ids` runs at project.rs:934, **upstream of the Phase-2
  routing branch**, so it is common to `Buckets` and `ParsedFold`. A missing
  fold-critical index (`tenders_procedure_key`) therefore costs **hours on every large
  re-fold**, not one slow endpoint. Cost-of-delay is far higher than priced.
- The ENOSPC objection that drove the rejection is answered *directly* by the pre-flight
  guard (6) — by making the build refuse safely, not by declining to self-heal. Once the
  build cannot fail silently, the argument for withholding the repair evaporates.

The original objection was sound on the evidence available; it was the *dichotomy* that
was wrong. Detect-report-and-repair keeps every part of the visibility case while
removing the human-in-the-loop dependency.

## Validation

- Unit: strip one deferred index on a scratch DB → `missing_deferred_indexes()` returns
  exactly it; run `Spec::Reindex` → empty.
- Boot: a DB with a stripped deferred index reports `/health` degraded naming that
  index **and enqueues exactly one `reindex` job**; a fully-indexed DB reports healthy
  and enqueues nothing. Assert **no index DDL runs during `init` itself** — the repair
  must be visible only as a queued job (the 82/83 regression guard).
- Dedup: two `init`s over the same unindexed DB (a restart loop) leave ONE queued
  reindex, not two.
- Pre-flight guard: with free space below the threshold, `Spec::Reindex` returns a loud
  distinct refusal and performs **no** index DDL; above it, it builds normally. This is
  the test that makes auto-repair safe, so it is not optional.
- **Alerting loop closed**: the standing alerting routine fires on
  degraded-for-missing-indexes and names the remedy. Acceptance is that an unindexed DB
  produces an *alert*, not merely a banner — the detector is only as good as its
  ability to reach someone who can act (the verification-input-discipline lesson: a
  check is only as trustworthy as its knowledge of whether its own output is seen).
- Generalise `rebuild_preserves_the_current_published_covering_index` (lib.rs:2149) from
  the one index to **all twelve**: after `strip → reset_tender_layer → build`, assert
  every name is present in `sqlite_master`. That test already exists for exactly this
  failure mode on one index; the point of this issue is that the *set* is what matters,
  not any single member.

## Note

Deliberately NOT folded into 64. Issue 64 is fold-index thrash — the reset-order /
DELETE-vs-live-index performance item on `reset_tender_layer`. This is deploy-time
migration tracking for a declared index set. Distinct problems on the same const.
Also not 82 — 82's specific fix (put `tenders_current_published` in the deferred set)
landed and is correct; this is the generalisation 82 stopped one step short of.

Root-caused independently by proj-fix and sdk-vendor, converging on the same mechanism
from different reads (proj-fix: exhaustive DDL-site enumeration; sdk-vendor: the
`if rebuild` gating + index-maintenance semantics).

## Design (proj-fix, 2026-08-03) — the builder exists; nothing schedules it

Grounding this in the code rather than describing a new subsystem: **`Spec::Reindex` in the supervisor
is already the builder we would otherwise write.**

```rust
Spec::Reindex => {
    self.db.build_organization_indexes().await?;   // CREATE INDEX IF NOT EXISTS loops
    self.db.build_tender_indexes().await?;
    let _ = self.db.checkpoint(CheckpointMode::Truncate).await;
}
```

Idempotent, durable across restarts (it is a queued job), sequential with the projection so it cannot
race a fold, and already paired with the TRUNCATE checkpoint that reclaims the build's WAL tail.

So the gap is **not a missing builder**. It is that nothing NOTICES an index is absent and asks for it.
That reframing makes 111 small and keeps it on machinery that is already proven in production, rather
than adding a second index-building path that would need its own hardening.

### Shape

1. **`Db::missing_deferred_indexes()`** — scan `sqlite_master` (one row per *object*, not per row of
   data) and return the deferred names that are absent. Committed in `eaf6346`.
2. The supervisor calls it at startup and, if anything is missing **and no `Reindex` is already
   pending**, self-enqueues one.
3. The existing queue runs it **in the background, after the service is up and serving**.

Boot is never blocked and nothing builds inline, so issues 82/83 cannot return — the boot cost is one
`sqlite_master` query. A fresh or test database has empty tables, so the job completes instantly;
tests that want indexes keep calling the builders directly as they do today.

Layering follows what already exists: `store` owns the index lists and the jobs table, so detection
lives there; `app` owns the lifecycle, so the enqueue sits in the supervisor — which is already the
thing that calls `build_*_indexes`.

Anti-drift: the org names moved into a `DEFERRED_ORG_INDEXES` const shared by builder and detector.
Two copies of that list would be a detector that can silently stop matching what is built — the
artifact-versus-proxy failure of issues 110/102 in a new place. `organizations_identity` is
deliberately excluded: it is built only when the table lacks the inline UNIQUE, so a database that has
the constraint legitimately lacks the index.

### Blocking unknown — scheduling is NOT yet written, deliberately

**Peak RSS while turso builds an index over 25.3M rows is unmeasured.** A b-tree build is a sort, and
whether turso 0.7.0 sorts in memory or spills decides the design:

- **spills** → backgrounding suffices and 111 is nearly done;
- **sorts in RAM** → a 25.3M-row build could OOM the box. Prior form: the issue-57 OOM and the 16 GB
  swap band-aid still in place, against a standing rule that full-corpus work keeps peak RAM flat and
  well under ~4 GB. The builder would then need id-range batching, or the index would have to stay a
  rebuild-time artifact.

run-driver has the time (47.6 s / 8.13M rows) and the disk (+145 MiB) but not the memory; asked for
`VmHWM` during the build. **One number decides it**, which is why `eaf6346` commits detection only and
stops short of scheduling.

Note their ~150 s prod extrapolation is explicitly a **floor** — their copy was fully cached, prod's
organizations live in the 453 GB file — so the background path is right regardless of the RSS answer.

### Why this matters beyond tidiness

Issue 117's DoS fix (`8cdc35d`) adds three deferred indexes. Without this, deploying it does not create
them, and `?country=`, `?kind=` and `tenders?source=` stay slow while the commit message says they are
fixed. A clock run in that window would correctly measure no improvement and look like a failed fix.
That is the concrete cost of 111 remaining open, and it is why 111 is the deploy blocker for 117 rather
than a follow-up to it.

## Resolved (`eaf6346` detection, `d018052` scheduling + cap)

At startup, after the durable queue is recovered, the supervisor asks
`missing_deferred_indexes()` what is absent and — if anything is, and no `Reindex` is already
queued — enqueues one. The existing job builds them on the worker **after the service is up**.
Boot cost is one `sqlite_master` scan; nothing builds inline.

### Why no batching, and what bounds the memory instead

run-driver measured a bulk `CREATE INDEX` on the deployed turso 0.7.0:

| approach | wall clock | peak RSS |
|---|---|---|
| bulk `CREATE INDEX` after load (8.13M rows) | 46.5 s | **366 MiB** |
| same at 25.3M rows | 413 s | **1.07 GB** |
| index created FIRST, rows inserted with it live (8.13M) | 6:33 | **24 MiB** |

Bulk is **O(rows), ~45 B/row, with no spill threshold**. Index-first is bounded — but only
available during a from-scratch rebuild, not retrofittable onto a populated table.

**And bulk cannot be batched.** `CREATE INDEX` has no range knob, and N partial indexes over
disjoint id ranges do not compose into one usable index: a partial index serves only queries whose
`WHERE` implies its predicate, so a filter carrying no id bound would use none of them. The
id-range fallback that was floated is not expressible.

So two things bound the memory:

1. **Sequential, never parallel.** Sequential keeps the peak at the largest *single* build rather
   than the sum — organizations + notices concurrently is ~2.3 GB against a ~4 GB ceiling on a box
   still carrying issue 57's swap band-aid. The job queue already serialises; the in-code comment
   exists so a later "optimisation" to fan out has to argue with the measurement.
2. **A row cap** (`MAX_AUTO_INDEX_ROWS` = 45M ≈ 2 GB at the measured constant, half the ceiling).
   `organization_mentions` (40.9M) passes; `changes` (93.6M, ~4 GB alone) would not, and is in
   neither deferred list — its indexes build index-first in the schema DDL or at projection end.

A refused index leaves a read slow and **says so on stderr**; an accepted one that does not fit
takes the process down mid-build, and **turso cannot interrupt a running statement**. So the cap
errs toward refusing, and uses `MAX(rowid)` — an O(1) seek rather than a `COUNT(*)` scan, and an
over-estimate once rows are deleted.

### Why the cap is a runtime check and not a compile-time assertion

The hazard is a **row count**, and row counts are not compile-time facts. A static allowlist of
table names would encode today's judgement about which tables are small and go stale silently the
moment one grows — which is exactly the mistake that put `notices(source, id)` in the schema batch
on the strength of a comment written when that table was 8× smaller. A compile-time check would
have caught a new *name*; it would not have caught the *growth*, and growth is what actually
happened.

### Boundary

Anything above the cap must be built **index-first at a rebuild**, or in a maintenance window —
never auto-bulk-built on the populated database. That is the line, and the cap enforces it rather
than documenting it.
