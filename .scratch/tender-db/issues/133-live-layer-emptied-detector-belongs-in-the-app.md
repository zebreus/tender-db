# 133 — detecting an emptied canonical layer belongs in the app, not in the snapshot gate

Status: open — proposed by sdk-vendor, filed by proj-fix (the app/supervisor/health surface is mine).
Not urgent; #28 proceeds with its own corrected claim meanwhile.
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
