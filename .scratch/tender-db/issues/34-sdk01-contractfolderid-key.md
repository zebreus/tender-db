# 34 — sdk-0.1 ContractFolderID as procedure key (island → merge upgrade)

Status: ready-for-agent

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
