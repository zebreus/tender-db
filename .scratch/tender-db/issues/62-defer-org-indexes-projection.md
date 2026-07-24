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

Commit: 4926149 (branch issue62-defer-org-indexes; source fix). Tests added on
top. Do NOT deploy from here — team-lead merges to main + deploys at a safe
boundary (after the current prod run finishes, or as a fallback if it trips).
