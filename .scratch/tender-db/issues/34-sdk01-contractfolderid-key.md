# 34 — sdk-0.1 ContractFolderID as procedure key (island → merge upgrade)

Status: needs-verification

Scoped out of issue 29: sdk-0.1's `SDK01-ContractFolderID` is not read
as a procedure key, so uuid-bearing sdk-0.1 CANs stay island Tenders
instead of merging with their TED / eForms-DE twin. CONTEXT.md/ADR-0003:
merge exactly on a strong explicit cross-reference — a shared procedure
uuid is that. The island design explicitly promises "upgradeable by
re-projection if linkage ever appears"; this delivers it for the ~40%
DÖE dialect.

Care points: only treat it as a key when it is a genuine uuid (the
sdk-0.1 numeric channel also publishes non-uuid folder ids — those stay
islands); a missed link must split, never wrongly merge (transitive-edge
rule); the TED↔DÖE merge-rate section of the data-quality tool is the
acceptance metric (currently 0/0 pre-backfill — expect a real rate in
the overlap window 2022-12→ after backfill + reprojection).

Acceptance: uuid-bearing sdk-0.1 notices merge with their TED twin on
reprojection (fixture pair test); non-uuid folder ids unchanged; merge
rate measurable and plausible post-backfill.

## Comments

### 2026-07-21 — implemented (needs-verification)

Fixed in `crates/ingest/src/project.rs`: procedure-key derivation now falls back,
for the sdk-0.1 dialect only, to `SDK01-ContractFolderID` when it is a **genuine
uuid** (`is_uuid`: 8-4-4-4-12 hex). BT-04 is still tried first; non-uuid /
numeric-channel folder ids return `None`, so those Tenders stay islands — a
missed link splits, never wrongly merges. Keying on the shared uuid makes a
uuid-bearing sdk-0.1 CAN group with its TED/eForms-DE twin through the existing
`procedure_key`-based identity path (ADR-0003), so the merge is automatic on
re-projection with no other change.

Tests (`tests/project.rs`):
`sdk01_uuid_folder_merges_with_ted_twin_but_non_uuid_stays_island` — the real
sdk-0.1 CAN (ContractFolderID 3d2aac86-…) + a TED twin publishing the same BT-04
uuid collapse into one Tender carrying both Sources; a non-uuid folder id stays
an island. Plus an `is_uuid` unit test. `cargo test -p ingest` green, clippy
clean (`-D warnings`); the issue-29 title assertion was made order-independent
(keying changed tender ordering — expected).

**Acceptance metric:** the data-quality tool's TED↔DÖE merge-rate section; 0/0
pre-backfill today, expect a real rate in the 2022-12→ overlap window after
backfill + re-projection. Not deploying — the boundary is the team lead's.
