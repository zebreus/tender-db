# 95 — ParsedFold's 5h07m CPU-bound zero-I/O stretch is unexplained

Status: open — known-unknown parked 2026-08-02 after the issue-85 re-fold was routed AWAY from this
path. NOT on the live path (the 90/91 routing fix sends incremental Phase-2 to Buckets), so it blocks
nothing operationally — but it gates the scoped-read optimisation and is an open hole in the model.
Kind: performance (investigation) — known-unknown
Blocked by: —
Relates to: 91 (read amplification — the I/O-bound sibling; this is its opposite), 90 (observability),
94 (the pre-pass whose scoped-read speedup this gates), 62

## Verify

    grep -c 'stage(&format!' crates/ingest/src/project.rs

- **done**: this record's foot names the stage that eats the CPU-bound stretch — the chase (a scoped six-figure `ParsedFold` on a scratch DB under the issue-90 stage timings) has been run; no command can print that, the foot is the evidence
- **open**: a positive count — the per-stage timing instrument of `project_incremental_chunked_phase2` is in the tree and ready (read 2026-09-19: `6`), the chase not yet run; `0` would mean the instrument itself is gone

## The unexplained observation

During the first eForms-DE 1.x incremental refold attempt, the `Phase2::ParsedFold` path ran for
**5h07m CPU-bound with near-zero I/O** before it was killed. That shape is the *opposite* of what the
path is supposed to be:

- ParsedFold is a **scattered read** of a scoped id set (`IN(512)` batches). The expected bottleneck is
  I/O — random re-reads (that is issue 91, read amplification). A scattered read that is CPU-bound with
  no I/O means the work is NOT where the model says it is.
- It is also NOT the fold quadratic (issue 92): proj-fix's run-driver code-read refuted O(chain²) by
  measurement on the live retire path, and the CPU-bound stretch preceded the apply phase.

So there is a CPU sink on the ParsedFold path that neither the read-amplification model (91) nor the
fold-quadratic model (92) accounts for. **We do not know what it is.** Candidate suspects, none
confirmed: plan-group construction over the touched set, an accidental O(n²) in touched-expansion
(`touched_existing_tender_ids` via `procedure_key IN`), surrogate-id assignment, or a mentions-pass
blow-up — but this is speculation until measured.

**Do not start from zero.** Issue 91's "What the diagnosis eliminated first" section already rules out
six candidates with their measured numbers and the exact prod DDL each was measured against. The
investigation begins from "these six are eliminated", not from scratch — reference that table.

## Why it is parked, not chased now

The 90/91 routing fix (`a55e599`) sends incremental Phase-2 with ≥100K planned notices to `Buckets`,
which is bounded, sequential, and terminating (issue 94's "not defects" section). Nothing on the live
path enters ParsedFold at scale anymore, so this stall cannot recur in production. It is a **known
unknown**, not a live risk.

## Why it still matters

It gates the scoped-read 10–30× win (issue 94): driving the pre-pass from the plan's ~473K ids instead
of sweeping all 14.2M is the obvious speedup, but it is ParsedFold-shaped. We will not deploy a
scoped-read pre-pass into an unexplained CPU-bound failure mode to save hours on a path that already
works. Explaining this is the precondition for that optimisation.

## The cheap, zero-risk chase

Run a scoped `ParsedFold` on a **scratch DB**, offline, under the issue-90 per-stage timings
(`stage(label)` elapsed seconds + per-batch heartbeat already in `project_incremental_chunked_phase2`).
The stage that eats the 5h will name itself the first time it runs. No prod risk, no fold in flight,
no deploy. Not urgent — do it in the hardening batch window alongside 92/93/94, or whenever the
scoped-read optimisation is next wanted.

**Critical repro requirement — the change-set must be SCOPED (six-figure), not a small delta.** Every
prior ParsedFold run that *completed* had a small delta and never reproduced the stall; it only appears
at re-fold scale. A quick small-delta repro will come back clean and prove nothing — it must drive a
six-figure change-set (comparable to this re-fold's 473,094 planned notices) to trigger the regime.
