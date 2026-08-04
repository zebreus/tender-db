# Daily-cycle integration for the standing structural gate (task #28, proj-fix half)

Status: DESIGN — not wired. Wiring gates on sdk-vendor's confinement measurement.
Owner: proj-fix (the cycle integration). The gate script itself is sdk-vendor's (`standing_gate.sh`).
Relates to: 28 (the gate), 107 (snapshot-freshness witness), 119, 23 (snapshots), 111 (reindex op)

## The cycle as it actually is

Read from `Supervisor::enqueue_daily` (`supervisor.rs`), not from memory:

```
09:35 Europe/Berlin  →  probe(ted) → process(ted)      [weekdays only]
                        probe(doe) → process(doe)
                        project {rebuild:false}
                        snapshot                        ← last, so it captures the fold
```

**There is no `reindex` step in the daily.** The cycle has been described in conversation as
"process → project → snapshot → reindex → VERIFY"; the reindex enqueued by issue 111 runs at **boot**,
not on the tick. Recorded because designing VERIFY to sit "after the reindex" would place it after a
step that never runs — and more importantly, because anyone believing reindex runs daily also believes
index completeness is re-checked daily, and it is not. Whether the daily *should* reindex is a separate
question, deliberately not answered here.

VERIFY therefore attaches after `snapshot`, which is already last.

## Why a systemd timer, not a supervisor job

The gate reads an **immutable snapshot**, so it cannot conflict with ingestion or projection for
*correctness* — the file's contents are already decided. The only conflict is I/O and page cache. That
makes "don't overlap ingestion" a **load** constraint, and systemd confines load (`MemoryMax`,
`IOWeight`, `CPUWeight`) better than the app can from inside its own process.

So the supervisor route's strongest claim — free serialisation — buys something obtainable another way,
while charging the failure mode that matters: a verifier that dies with the supervisor cannot report on
a wedged supervisor, which is exactly the condition its staleness check exists to catch. The asymmetry
decides it: `job_log` visibility is recoverable from the timer side (POST the verdict), surviving a
wedged supervisor is not recoverable from the other side.

## The pinned input, and the pointer that has to exist

sdk-vendor's gate now takes `SNAPSHOT=<path>` and carries `mode=pinned` vs `mode=newest` into its
verdict, because "newest" cannot establish *which run* wrote the file: a failed snapshot step leaves
yesterday's file as newest, still inside `MAX_AGE_H`, and the gate would verify it green and report
success for a cycle that produced nothing.

Pinning requires the pipeline to publish the path it just wrote. It does not today — `snapshot::run`
builds `dest`, takes the snapshot, prunes, and returns a summary **string**. Nothing durable names the
file.

**Proposed (my change, app side):** on success only, `snapshot::run` writes the absolute path to
`<dir>/latest` via write-temp-then-`rename` (atomic; a reader never sees a half-written pointer). The
timer reads that file and passes it as `SNAPSHOT=`.

Three properties that make this the right shape:

- **Written only after the snapshot succeeds**, so a failed snapshot never advertises a path. The
  pointer going stale IS the failure signal, and it is the signal the gate already knows how to report.
- **No fallback to "newest" if the pointer is missing or stale.** Falling back is precisely the hole
  sdk-vendor just closed; the verifier reports STALE and stops instead of guessing.
- **Prune-safe by construction.** The local ring keeps 2. If a prune deletes the pinned file mid-run,
  an already-open fd keeps it readable to completion on Linux — so the run finishes on the file it
  started, and the *pointer*, not the data, is what may have moved.

## The interlock: bounded deferral, never a precondition

Load, not correctness — so the interlock must never be able to suppress the verifier indefinitely:

1. Ask the app whether a heavy job is in flight (short timeout).
2. If yes, defer and re-ask, up to `MAX_DEFER` (proposed 90 min).
3. Past the bound, **run anyway** and mark the verdict `contended`.
4. If the app is **unreachable**, do NOT defer — run immediately. An unreachable app is the wedge case
   the timer exists to survive, and there is no contention to avoid when nothing is running.

The failure this avoids: gating VERIFY behind "no heavy job running" *unconditionally* means a wedged
pipeline (a heavy job stuck forever) suppresses the verifier at exactly the moment its staleness check
is the thing that would tell us. A verifier that goes quiet when the system breaks is the
decorative-detector shape wearing a different hat. Under-confined and noisy beats silent.

## Verdict vocabulary — a degraded run must not read as a clean one

`PASS` · `FAIL` (violations) · `STALE` (pointer missing/stale, or input older than the bound) ·
`CONTENDED` (ran under load; timings untrustworthy, violations still valid) · `ERROR` (could not run).

`CONTENDED` exists because a run that had to fight the disk is not equivalent to a quiet one, and
silently collapsing the two is how a caveat gets lost. sdk-vendor is carrying the marker in the gate's
output.

## Publication

- **journald always** — the durable channel, and the only one that works when the app is wedged.
- **Best-effort POST to the app** so the verdict reaches `/admin` and the dashboard.
- A failed POST changes the *reporting*, never the verdict, and says so in the journald line.

## Confinement

`MemoryMax` (so the scan reclaims its own page cache instead of evicting the live service's ~2 GB),
plus `IOWeight`/`CPUWeight`/`Nice`. **Values deliberately absent**: they are sdk-vendor's measurement to
make, and inventing numbers here would be the same species of unmeasured assumption this whole task
exists to remove. Wiring waits on that measurement.

## Failure semantics

The verifier is **downstream and read-only**: a FAIL never blocks or reverts the pipeline. It alerts.
The pipeline has already committed by the time the gate reads the snapshot, so there is nothing for it
to stop — its product is knowledge, not control.

## What a green does not mean

Two limits, both already written where the green is produced (sdk-vendor's "WHAT A GREEN DOES NOT MEAN"
header):

- **Vacuity** — 0 violations is trivially true of an empty table; the `*_present` checks close it.
- **A check downstream of a guard cannot validate that guard.** For `head_not_max` specifically, my
  task-#27 assertion refuses and rolls back, so a violation never lands, so the snapshot is clean and the
  gate finds nothing. Gate-green there means "no damage **or** damage prevented" — indistinguishable.
  The signal that a violation occurred is a failed projection job, nowhere else.

## Before wiring

1. sdk-vendor's confinement measurement (blocks wiring, not design).
2. The `latest` pointer lands in `snapshot::run` (mine, small, testable: a failed snapshot must leave
   the pointer untouched — that is the test worth writing first).
3. Where the dashboard renders the verdict — a surface question for team-lead, not a mechanism one.
