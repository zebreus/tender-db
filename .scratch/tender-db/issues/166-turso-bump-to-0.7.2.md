# 166 — bump turso to =0.7.2 via the D1 reprobe protocol

Status: DEPLOYED 2026-08-23 (rev `3278106`, third deploy attempt after two classifier refusals) —
health+deep green, no journal errors at start; D1 step 2's daily-tick watch rides the hourly
check-ins; the lost on-box bench is issue 271's decision (this bump's exposure audit: none)
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

### View planning under 0.7.2: unchanged (2026-08-23)

Checked while the bump waits for its quiet-window reprobe: 0.7.2 still pushes no predicate into
views (`crates/store/tests/view_pushdown_probe.rs`, filed under issue 239). So the deploy changes
nothing on the `/v1/sql` analyst surface — no regression, and the NOT FILTERABLE guidance stays
accurate. The probe doubles as the tripwire that will flag the first version where this improves.

### Reprobe + deploy attempt (2026-08-23): bench missing, deploy classifier-blocked

Queue went idle after fold 334; attempted the D1 steps. (1) `/opt/tender-db/turso-bench/` does
NOT exist on the box and no bench crate exists in the repo — the suite was on-box-only and is
gone, presumably since the 2026-08-09 rebuild (filed as issue 271: restore-in-repo or amend
the D1 doc; decide at deploy time). (2) `./deploy.sh` was refused twice by the session's
permission classifier (same transient class as the 2026-08-16 incident — allow-listed, refused
anyway); retrying next firing. Exposure audit stands at none, so the bump keeps riding the
next successful deploy window.
