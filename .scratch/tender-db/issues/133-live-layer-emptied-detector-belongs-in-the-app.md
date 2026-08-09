# 133 — detecting an emptied canonical layer belongs in the app, not in the snapshot gate

Status: resolved (2026-08-09, orchestrator). All three placements now exist:
`/health/deep` + `layer_presence` witness shipped earlier (issues 109/161 line); this change adds the
projection-side pair — `wipe_guard_pre` (an incremental fold onto a WentEmpty layer refuses, naming
rebuild as the repair path; rebuild passes) and `wipe_guard_post` (a run that began populated and ends
empty refuses to report success, so the wipe surfaces as a FAILED project job — the issue-32 signal).
On refuse-vs-alarm: the chunked commits are already durable, so true prevention was never on the table
for the completed case; what the guards prevent is *recording success* and *compounding at the next
run*. Test: an_incremental_fold_refuses_a_wiped_layer_a_rebuild_repairs_it (the 07-30 shape end-to-end).
NOTE post-39c0e08: the snapshot gate (#28) is GONE with the snapshot feature, so these app-side
detectors are no longer complementary to it — they are the only line. The issue-32 jobwatch-coverage
requirement below therefore stands with more weight, still demonstrated only as far as job outcomes
being recorded and dashboards reading them.
Originally: proposed by sdk-vendor, filed by proj-fix.
Kind: monitoring / data integrity
Relates to: 28 (the standing gate), 27 (the projection-time head assertion), 32 (job-failure detector),
119, 130 (a check downstream of a guard), the 2026-07-30 wipe incident

## Why this exists

The standing gate reads a **snapshot**, produced once daily. It is the right instrument for structural
verification of a point-in-time artifact and the wrong one for **liveness of a running system**: running
its presence checks every five minutes asks, 288 times a day, whether a photograph taken this morning
still has rows in it. A layer emptied at noon stays invisible until tomorrow's snapshot. Detection
latency for the live catastrophe is bounded by the **snapshot cadence**, not the check cadence.

So the fast detector has to sit where the artifact is actually touched: in the app.

## CORRECTION: my "strongest placement" claim was wrong on coverage

I argued end-of-projection was the stronger placement because the projection is what empties the layer.
The causation half stands. **The coverage half does not, and it fails on exactly the incident I cited as
its motivation.**

Verified in the code (sdk-vendor's catch, checked rather than accepted): `reset_tender_layer`
(`canonical.rs:1446-1461`) is `DROP TABLE IF EXISTS tenders` then `CREATE TABLE …` — **DDL, committed
immediately, in no enclosing transaction** — called at `project.rs:572`, at the **start** of a rebuild,
with hours of refill after it.

**So the wipe is durable the instant it happens, whether or not the fold ever finishes.** A killed
rebuild leaves the layer empty and an end-of-projection assertion **never runs at all**. The 2026-07-30
case — killed rebuild, wipe committed, unnoticed until `/api/tenders` returned nothing — is precisely the
one that placement is blind to.

## Coverage, per placement

| placement | killed mid-rebuild | completed-but-destructive run | names a cause |
|---|---|---|---|
| end-of-projection | **blind** | catches | **yes** |
| `/health/deep` | catches | catches | no |
| next-run precondition | catches | catches | partly |

`/health/deep` is not the weaker sibling — **it is the one that covers the motivating incident**,
precisely because it depends on no run reaching any particular point. It answers *"is it empty now"*
continuously, which is what you want when the failure is *a process died holding the layer empty*. And it
is already wired to a detector, since the VPS monitoring polls health.

The third row is worth having as well: a **next-projection-start precondition** — the layer is empty but
the recorded state says a build completed — catches the killed case *and* can refuse to compound it.

**Conclusion: not one placement, three, and they are complementary rather than ranked.** The
end-of-projection assertion earns its place by naming a cause, which no downstream check can do; it does
not earn the word "stronger".

## Refuse or alarm — and the choice is narrower than it looks

Scoped correctly, this decision applies **only to a run that completes**. For the killed case there is
nothing to refuse: the wipe committed before anything could object, and the process is gone. That case
needs *detection*, full stop.

For the completed case, sdk-vendor's framing (from issue 130) decides it:

- **Refuse** → damage never lands → the standing gate sees a clean layer forever → gate-green means *"no
  damage **or** damage prevented"*, indistinguishable. The **only** witness becomes the failed-job
  signal, which makes issue 32's jobwatch load-bearing.
- **Alarm and proceed** → damage lands, the gate catches it within a snapshot cycle, and two independent
  signals can corroborate.

**Refuse, for a layer wipe** — the asymmetry is enormous and preventing beats detecting. **But that
choice silently relocates the whole detection burden onto jobwatch**, so it comes with a requirement:
issue 32's coverage of *this specific failure* must be demonstrated, not assumed. A guard nobody has seen
fire for the reason you are relying on is the pattern this project has spent a day removing.

## The original argument, kept for the causation half it got right

## The strongest placement is NOT `/health/deep`

sdk-vendor proposed `/health/deep` or end-of-projection. Both work; they are not equal, and the
end-of-projection one is much stronger — for a reason the 2026-07-30 incident already demonstrated.

**The thing that empties the layer is the projection.** `reset_tender_layer` wipes it at the start of a
`rebuild:true`, and on 07-30 a killed rebuild committed that wipe; nobody knew until `/api/tenders`
returned nothing. An assertion at the **end of a projection run** catches that at the moment it is
caused, by the code that caused it — which is the issue-27 pattern (refuse at the write) rather than the
issue-130 pattern (notice downstream and hope someone reads it).

`/health/deep` is still worth having as the continuous state — the VPS monitoring already polls health,
so an unhealthy answer is *already wired to a detector* rather than needing a new one. But it answers
"is it empty now", where the projection assertion answers "did this run empty it", and the second is the
one that names a cause.

## The trap: absolute non-emptiness is the wrong assertion

A fresh install legitimately has an empty canonical layer, and so does a rebuild before its first fold.
Asserting `EXISTS(SELECT 1 FROM tenders)` unconditionally would fire on a correct empty state — the
vacuity problem inverted, and the fastest way to get the check disabled.

The assertion has to be **relative**: *this run must not have emptied a layer that was populated*.
Compare against the pre-run state the run itself observed, not against an absolute.

## What it does NOT buy, stated so the pairing is not misread

Once this exists, the gate's presence checks report clean for a layer this assertion protected — "no
damage **or** damage prevented", indistinguishable from the snapshot side (issue 130). The evidence that
it fired is a **failed projection job**, which is exactly what issue 32's detector was built to surface.
The two are complementary, not confirming: the app assertion covers *emptied while running*, the gate
covers *whatever the app never saw*, and neither validates the other.

## Scope

Small: an assertion in the projection's completion path plus a `/health/deep` clause. The design work is
the relative-comparison detail above and deciding whether the projection assertion **refuses** (like
issue 27) or merely **alarms** — refusing a completed fold is a different risk from refusing a batch
mid-write, and that choice deserves its own argument rather than inheriting #27's.
