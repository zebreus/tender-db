# 166 — bump turso to =0.7.2 via the D1 reprobe protocol

Status: open — NOT urgent (exposure to the 0.7.1 corruption fix verified none)
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
