# 498 — a failed flush or COMMIT leaves the shared writer connection inside its transaction

Status: done (watch) — units 0, 1, 2 and 3 DEPLOYED 2026-10-09 (`ad4fddd`, health ok). NEXT: on 2026-10-16, grep the
journal for `INSIDE an open transaction`, then close. Filed 2026-10-08 from the issue 495 unit 2 planning review (`wf_da3c04cf-2f7`, step 12),
confirmed against the code.
- Unit 1 (census) DONE 2026-10-09: 22 confirmed leaking sites, ranked, plus the helper design —
  `.scratch/tender-db/498-tx/census-2026-10-09.{md,json}`.
- Unit 0 (defense in depth, the census's recommendation) BUILT 2026-10-09: `Db::conn_for` and the token touch's
  `try_lock` (the one acquisition outside it) pass the guard through `end_dangling_transaction`, which rolls back a
  transaction a previous holder left open and logs `[store] the writer was handed to <file:line> INSIDE an open
  transaction`. It is the only fix for the panic and cancelled-future cases. Test:
  `the_writer_is_never_handed_over_inside_a_transaction`. DEPLOYED 2026-10-09 00:34 UTC (`90fb19d`, gate green, health ok).
- Units 2 and 3 BUILT 2026-10-09: `crates/store/src/tx.rs` (`Db::immediate`, `Db::within` for the two plain-`BEGIN`
  sites, and `finish`, which replaces canonical.rs's `finish_tx`), with five tests: a failing body, a failing
  COMMIT (deferred FK), an engine-ended transaction (`RAISE(ROLLBACK)`), a failing BEGIN, and success. All 22
  census sites are converted; the goldens are unchanged, so the success path is byte-identical.
- Review `wf_a128d5c8-b83` (four lenses with adversarial verification). Success-path identity was confirmed
  literal by literal, and all 22 sites are complete. It found that an `async fn` helper held an unboxed body
  about 3× in every caller's future (a 4.4 KB body made a 13.6 KB future and a ~70 KB O0 poll chain). That
  threatened issue 467's budgets on `run_project`'s chain, so `ad4fddd` makes `within` a plain fn that boxes the
  body (one allocation per transaction). The same review found the resolver's lenient probes, filed as
  issue 501.
- DEPLOYED 2026-10-09 ~01:20 UTC (`ad4fddd`, gate green with the size-budget test, health ok).
- NEXT: a journal grep for `INSIDE an open transaction` on 2026-10-16. A hit means a panic or a cancelled future,
  the only leaks left. The census's already-safe hand-written sites (about 30) may follow for uniformity; they
  carry no risk.
Kind: robustness / the single writer (`Db::conn`)
Relates to: 241 / 256 (the writer queue), 323 (`checkpoint_on` inside a transaction), 495 (unit 2 rewrites the
same batch loop, but must stay byte-identical, so the fix is not part of it)

## What is wrong

`apply_tenders` (`crates/store/src/canonical.rs`, around line 15316) rolls back when `apply_tender_tx` fails
and when `assert_heads_match` fails. It does not roll back in two other cases:

- `applied.leaf_rows += pending.flush(&conn).await?;` — any error in the batched leaf INSERTs.
- `conn.execute("COMMIT", ()).await?;` — a failed COMMIT.

Either error returns through `?` with the connection still inside `BEGIN IMMEDIATE`. That connection is the
process's ONE writer, so the next `Db::conn` holder inherits an open transaction:

- Its own `BEGIN IMMEDIATE` fails with "cannot start a transaction within a transaction".
- Its autocommit writes run INSIDE the dangling transaction. The presence observer's 5-minute `UPDATE`
  (issue 241's waiter) is one example. They commit only if some later caller happens to issue a COMMIT, and
  that would commit the half-written fold batch with them.
- Otherwise every write fails until a restart, which then discards everything.

turso aborts only the failing statement (`abort` → `end_statement`), not the transaction, so nothing
cleans up by itself.

Likely triggers: disk full (SQLITE_FULL), an I/O error, or a constraint violation from a future schema
change. Rare, but the failure would be a wedge, not one failed job.

## Scope

There are 59 `"BEGIN IMMEDIATE"` sites (canonical.rs 49, lib.rs 7, accounts.rs, jobs.rs and rates.rs one
each), and no shared transaction helper. Each site hand-writes its own error handling. The two above were found
by reading one loop. The others are unaudited.

## Units

1. **Census.** For each site, list the fallible awaits between BEGIN and COMMIT/ROLLBACK that return through
   `?` without a ROLLBACK, and the COMMITs whose failure is not rolled back. Rank them by how often they run
   (apply_tenders, resolve_mentions, insert_plan and the merges first).
2. **One helper.**
   - Add `Db`-level `async fn immediate<T>(conn, f)`: BEGIN IMMEDIATE, run the body, COMMIT; on ANY error
     from the body or the COMMIT, `ROLLBACK` (best-effort) and return the original error.
   - A test makes a body fail mid-way, and also a COMMIT fail (for example a deferred FK violation, or a
     `RAISE` in a temp trigger). Then:
     - the connection is in autocommit again (`is_autocommit()`);
     - the next BEGIN succeeds;
     - none of the failed body's rows are visible.
3. **Convert the sites** the census ranks, hottest first. Each conversion is byte-identical on the success
   path. apply_tenders goes after issue 495 unit 2 lands, or in the same branch, so the two do not collide in
   the batch loop.
