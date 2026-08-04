# 119 — the snapshot ring has silent cadence gaps, and nothing asserts freshness

Status: REWRITTEN 2026-08-04 — original premise FALSIFIED (see below). Folded into 107;
the gate (#28 `standing_gate.sh`) is the fix for the assertion half. Kept open for the
cadence half.
Kind: verification (input freshness is unasserted)
Relates to: 107 (the parent — a verify must prove its input is the input it thinks),
111 (the shape the original filing borrowed, wrongly), 112 (section A), 121, task #28

## Correction first: the original defect statement was false

This issue was filed as *"section A's freshness depends on a snapshot nothing
guarantees"*, on the finding that there was **"no cron, timer or `/admin` route"**
producing snapshots.

**There is a producer, and there was one when this was filed.** It is the daily
supervisor job: `supervisor.rs:enqueue_daily()` ends with `Spec::Snapshot`, placed last
in the pipeline so it captures the freshly folded layer. Live since `83edbea`
(2026-07-29) — **five days before this issue was filed.** Found independently by
sdk-vendor (reading `enqueue_daily`) and run-driver (tracing the producer to the
scheduled tick); the lead confirmed and corrected the memory note.

**Why the original search missed it is the reusable part.** Cron, timers and `/admin`
routes are the three places this project's convention says ops logic *is not* — the
`no-dev-shortcuts-in-prod` rule puts ops logic *in the app*, under the supervisor. The
search was thorough and looked in exactly the wrong three places, because it was
looking for the shape the problem would have had in a differently-built system. A
negative result from a search is only as good as the search's model of where the thing
would live.

Two consequences follow, and both dissolve the original framing:

* **This is not 111's shape.** 111 was "a thing the system depends on with no component
  whose job is to produce it". Here a component's job *is* to produce it. The borrowed
  sentence was the most confident part of the filing and the wrongest.
* **The capacity objection dissolves too.** `/data` is XFS `reflink=1`; `filefrag -v`
  reports `shared` on every extent of the current snapshot, and apparent sizes sum to
  ~2.1 TB on a 1000 GB filesystem. `std::fs::copy` → `copy_file_range` → XFS remap: a
  snapshot is a reflink, near-zero in space and I/O. "A copy needs another ~453 GB" was
  false. (See 121; memory corrected by the lead.)

## What is actually broken

Not production, and not the producer. **Two narrower things:**

1. **Cadence has silent gaps.** Scheduled is not guaranteed: the ring shows **Jul 30 and
   Jul 31 missing**. The snapshot is the last step of the daily pipeline, so it is only
   as reliable as the pipeline reaching its end — a wedged or failed ingest silently
   produces no snapshot, and nothing says so.
2. **Nothing asserted freshness.** Every consumer read whatever file was newest and
   reported on it. A snapshot that happens to be recent gives a correct verdict; the
   moment the artifact under test is newer than the last snapshot, the gate reports the
   opposite of the truth. Section A now prints its snapshot's path and age (`8b6a2d6`) —
   but **declaring staleness is not preventing it**, and that gap is this issue.

This is 107's sentence exactly — *a verification must prove its input is the input it
thinks it is* — which is why it now lives under 107 rather than beside it.

## The fix for (2) exists

`canonical-verify/standing_gate.sh` (task #28) **is** the freshness assertion: it states
the snapshot's path, size, mtime and age, and **refuses** — `exit 2`, `VERDICT
stale_input` — past `MAX_AGE_H` (default 30h, one missed daily). Exercised in isolation:
a 40h-old input is refused, not verified.

It also closes the subtler variant proj-fix identified: resolving "the newest snapshot"
cannot establish the file is *this cycle's* output. If the snapshot step fails, the
newest file is yesterday's, still inside the age bound, and a green verdict would report
success for a cycle that produced nothing. So the gate takes a **pinned** `SNAPSHOT=`
path from the pipeline and records the resolution mode in its verdict — a green from a
guessed path cannot be mistaken for a green from a pinned one.

## What remains open

**The cadence half (1).** A missed daily still silently yields no fresh snapshot. The
gate now makes that *loud at the consumer* (a stale input is refused rather than
verified), which is the important half — but it does not make the producer more
reliable, and a consumer that only refuses is not the same as a ring without holes.

Options, unresolved:

1. **Alert on the gap directly** — the VPS monitoring built in task #23 already watches
   `/data` free space via a systemd timer → journal; snapshot age is the same shape and
   the cheapest addition.
2. **Decouple snapshotting from the ingest pipeline** — a reflink snapshot costs
   near-nothing (above), so the argument for keeping it as the pipeline's last step is
   weaker than it was when it looked like a 453 GB copy. A timer could take one
   regardless of whether the daily completed.
3. **Leave it** — the consumer-side refusal is arguably enough, since the failure is now
   loud where it matters. Cheapest, and defensible if the gaps stay rare.

Recommend (1) now and (2) only if gaps recur: (1) is small and additive, while (2)
changes when snapshots are taken relative to the fold and needs thought about capturing
a half-folded layer.
