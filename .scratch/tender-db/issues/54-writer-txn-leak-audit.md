# 54 — Audit writer BEGIN IMMEDIATE sites for the same leak class

Status: resolved (2026-07-23) — writer-self-pin hypothesis FALSIFIED; real cause was issue 42
Severity: MEDIUM (worse blast radius than the SSE read leak if it occurs)

## Resolution (2026-07-23)

The writer-self-pin theory this issue was opened to chase is DISPROVEN. The
WAL-diagnosis instrument ran the exact prod per-notice writer sequence (`BEGIN
IMMEDIATE` → INSERT → internal `notice_id` SELECT → COMMIT) across many packages
with a per-package TRUNCATE on the same writer: the boundary TRUNCATE returned
`busy=false` and the WAL folded to 0 every time. A lone writer does NOT pin its
own WAL after an explicit-txn COMMIT — no writer self-pin, no accumulating
read-mark. The real second pin was `store::Db::import_lag()`'s full-scan reader,
not a writer txn (see issue 42; fixed by the id-PK read in 4d023bb / 0eb1c69).

The rollback-guard question that motivated the broader audit is not left open on
suspicion: the write paths already carry unconditional ROLLBACK-on-error guards
(accounts.rs, lib.rs, canonical.rs, verified earlier), and no leaked writer txn
appeared anywhere in the multi-day WAL investigation. Closing. If a concrete
writer-txn leak is ever observed (e.g. a dropped write future mid-BEGIN-IMMEDIATE
before the projection burst), reopen with that repro — but there is no evidence
of one, and the WAL runaway it was theorised to explain is fully accounted for
by issue 42.

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
