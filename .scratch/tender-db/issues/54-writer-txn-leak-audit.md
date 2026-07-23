# 54 — Audit writer BEGIN IMMEDIATE sites for the same leak class

Status: ready-for-agent
Severity: MEDIUM (worse blast radius than the SSE read leak if it occurs)

Found while fixing issue 53's WAL pin (a pooled READER returned mid-open-
transaction froze a snapshot). The pool-level discard fix (5a8bea3)
covers the reader pools, but NOT the single writer connection. The writer
has its own `BEGIN IMMEDIATE` sites (crates/store/src/jobs.rs,
crates/store/src/lib.rs, canonical.rs). A leaked/abandoned writer
transaction is a DIFFERENT, worse failure mode: it blocks all WRITES
(not just the checkpoint), and the writer isn't request-cancellable the
way SSE is — but a dropped write future (turso 0.7 "poisoned
transaction" trap, CONTEXT.md) mid-BEGIN-IMMEDIATE could leave it open.

There IS a ROLLBACK-on-error guard on the write paths (verified earlier:
accounts.rs, lib.rs, canonical.rs) — this audit is to confirm EVERY
BEGIN IMMEDIATE site has an unconditional rollback on every error/drop
path, especially before the projection (job 5) runs its long batched
transactions. Task: audit each writer BEGIN IMMEDIATE, confirm the
rollback guard is airtight (no early-return / `?` between BEGIN and the
guard), add a test for any gap. Do before the projection.

Acceptance: every writer BEGIN IMMEDIATE has a proven unconditional
rollback on all exit paths; no writer-txn can leak.
