# 62 — Defer Organization indexes on the full-rebuild projection

Status: in-review
Severity: HIGH (superlinear Phase-1 at prod scale; disk saturation + health starvation)
Blocked by: —
Relates to: 60 (third and final scattered-index sub-fix), 59, 61

The third of the three scattered-index seek-explosion bugs in the Phase-1 plan
build (issue 60). The first two — `plan_ojs_node` PK and the `organizations`
inline UNIQUE — were handled under 60; this one carries the remaining fix.

## Root cause

On a full-rebuild projection, Phase 1 inserted, per row, into two indexes whose
key is NOT monotonic with insert order:

- `organizations_identity` UNIQUE(country, identifier_kind, identifier) — a new
  org's identity is scattered w.r.t. its autoincrement id, so the uniqueness
  probe + insert lands at a random b-tree position.
- `organization_mentions_org` ON (organization_id) — a mention reuses an *older*
  org, so the id it indexes is scattered w.r.t. the mention's sequential PK
  (~30M+ rows — the larger of the two).

Both thrash once the index outgrows the page cache: random b-tree seeks →
disk saturation → superlinear slowdown → health starvation.

Prod demonstrated it unambiguously at ~4M notices: per-500k interval hit 591s
(×1.78 steepening), disk 99.9% util, ~3028 random reads/s at 5 KB, /health
starved (000 over 20s). It self-limited as the new-org creation rate fell —
which is exactly what implicates this insert path.

## Fix

On `rebuild:true` only, the projection now:
1. `strip_organization_indexes` — after `clear_canonical`, DROP + recreate
   `organizations` and `organization_mentions` as BARE tables (no identity
   index, no org-id index). Drops the whole table, so it works whether the DB
   still has the old inline-UNIQUE auto-index or the new named index.
2. Phase 1 bulk-loads orgs by sequential autoincrement id and mentions by
   sequential PK(notice_id, section_id) — pure appends, no random index
   maintenance. The in-RAM `org_of` map is the run's authoritative dedup, so no
   duplicate identifiers are ever emitted.
3. `build_organization_indexes` — after Phase 2, build both indexes ONCE,
   sorted. Strictly less work than the millions of random inserts it replaces;
   the `org_of` dedup guarantees the unique index builds without conflict.

The `organizations` identity uniqueness moved from an inline `UNIQUE` constraint
to a NAMED `organizations_identity` index so a rebuild can drop + rebuild it.

The incremental (`rebuild:false`) path is UNCHANGED — low volume, no thrash,
uses the schema-created indexes.

## Schema migration

- Fresh DB: created straight into the bare-table + named-index shape.
- Existing prod DB (old inline UNIQUE + its auto-index): the first `rebuild:true`
  recreates the org tables bare, dropping the stale auto-index; then rebuilds the
  named indexes. Between deploy and that rebuild, the DB transiently carries both
  the old auto-index and the new named index — harmless (daily `rebuild:false`
  runs are low-volume), and it resolves exactly when the rebuild that matters
  runs. Pinned by `crates/store/tests/org_schema_migration.rs`.

## Verification

- Output byte-identical: `cargo test -p ingest --test project --test
  project_equivalence` green — the deferred index rebuild finds no conflict
  (`org_of` authoritative).
- Schema end-state (fresh + existing→reconciled): `org_schema_migration.rs`.
- Flat-time (bounded-cache analogue of `plan_bulk_load.rs`):
  `org_index_bulk_load.rs` — NEW deferred load stays flat; the OLD indexed
  pattern's full thrash only manifests at the 254GB/12.4M regime a laptop can't
  reach (prod's 591s intervals are the real evidence), so the local test bounds
  the NEW-path flatness and characterizes the OLD slope. See numbers in the
  commit / test output.

## ROOT CAUSE CONFIRMED (2026-07-24) — read-seek, cache-masked at laptop scale

Wall-clock flatness at 96k–320k was AMBIGUOUS (could be "no reads" or "reads
served from OS page cache in µs"). Disambiguated by counting turso's own read
bytes (`/proc/self/io` rchar) under a 256 KiB cache — `org_index_read_
amplification.rs`, 120k inserts:

  A bare-sequential append : 0 B/row       (baseline — appends to one hot leaf)
  B non-unique scattered   : 29 B/row = 414× A
  C unique scattered       : 29 B/row = 414× A, and 1.00× B

So a scattered-key index insert DOES a random READ per row — a b-tree traversal
to the target leaf — 414× the sequential baseline. Confirmed in turso 0.7.0
source: a UNIQUE insert emits an `Insn::NoConflict` probe (seek) per constraint
(`translate/insert.rs::emit_preflight_constraint_checks`), and even a non-unique
`Insn::IdxInsert` uses `require_seek()` to find its leaf. That read is served
from cache when the index is small (why wall-clock is flat here and the absolute
volume is only 3.28 MiB), but becomes a DISK seek once the index working set
outgrows the cache — prod's 3028 random reads/s at 4M notices.

Two consequences for the fix:
- The UNIQUE constraint's probe is NOT an extra cost beyond the position-seek
  (C = 1.00× B). It is the b-tree leaf-seek both index types do. So the earlier
  "organizations UNIQUE ruled out (measured flat)" was a wall-clock artifact
  (cache-masked); the read IS there. Deferring BOTH indexes is correct.
- Per-row read cost is equal, so `organization_mentions_org` (~30M mentions >
  ~12M orgs) is the larger AGGREGATE read source — matches the observed self-
  limiting as the new-org creation rate fell late in the corpus.

CAVEAT: this is ONE of (at least) two Phase-1 random-read sources on a full
rebuild; the other is the parsed-layer scattered reads (issue 60, addressed by
`cache_size`). The laptop test cannot apportion prod's 3028 reads/s between them.
62 provably removes the org-index share (bare bulk-load reads 0; the one-time
sorted rebuild reads sequentially).

Commit: 4926149 (source fix) + 008b501 (tests + this issue) on branch
issue62-defer-org-indexes. Do NOT deploy from here — team-lead merges + deploys
at a safe boundary, and is weighing 62's schema complexity vs its share of the
read load and vs issue 58 (incremental — the bigger structural lever, since even
daily projections currently re-read the whole 12.4M corpus).
