# 166 — bump turso to =0.7.2 via the D1 reprobe protocol

Status: in-implementation (2026-08-22, owner) — bump landed on main, all suites green locally; on-box probe suite + kill-9 crash loop + deploy PENDING the next quiet window (text-era campaign pipeline running)
Role: run-driver

From the 2026-08-09 upstream-drift audit (docs/research/upstream-drift-2026-08.md).

turso 0.7.1 (2026-07-22) fixes #6858: `UPSERT … DO UPDATE` deletes
secondary-index entries before validating the replacement row; an update arm
aborting on a unique violation leaves persistent table/index inconsistency.
0.7.2 (2026-07-30) adds sync/MVCC fixes irrelevant to our file-backed use.

**Exposure audit (2026-08-09): none.** The codebase's only
`ON CONFLICT … DO UPDATE` is the `layer_presence` upsert
(store/src/canonical.rs:3760); that table is `name TEXT PRIMARY KEY` plus
three non-unique columns, no secondary indexes — the failure shape cannot
occur. 0.7.1's IN-list planner fix is also not load-bearing for us (no large
IN lists on hot paths).

Still worth taking on the next quiet day, per the D1 pinning policy
(docs/research/turso-scale.md): bump `=0.7.0` → `=0.7.2`, rerun the probe
suite + kill-9 crash loop, deploy with the ordinary gates. The pin exists so
bumps are deliberate, not so they never happen.

Explicitly NOT: 0.8.0-pre (recursive CTEs, window functions, FTS perf land
there, but none of our pain points — interrupt() exposure, VACUUM INTO OOM,
write-drop poisoning — are fixed, and it is a pre-release).

## Bump landed locally (2026-08-22, owner)

`=0.7.0` → `=0.7.2` on both pins (`turso` and `turso_parser` move together — the parser is the
/v1/sql allow-list wall and must match the engine's AST exactly). `cargo update -p turso -p
turso_parser` moved the whole turso family (core/ext/macros/sdk_kit/sync) to 0.7.2 and nothing
else. `ops/check.sh`: **all suites green in 615s** — the EQP gate tests in particular, which pin
planner behaviour and would catch a 0.7.x planner change (the 0.7.1 IN-list planner fix did not
disturb them).

Remaining, per the D1 protocol, when the queue is idle (campaign fold 334 must land first):
1. On-box `/opt/tender-db/turso-bench/` rerun — probes + kill-9 crash loop at minimum — against a
   0.7.2 build of the bench binaries.
2. Deploy with the ordinary gates; watch the journal through a daily tick.
The upsert-corruption fix (#6858) we carry no exposure to, so there is no urgency ordering this
above campaign work; it rides the next natural deploy window.
