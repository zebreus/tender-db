# 501 — a lenient probe error can leave the resolver writing in autocommit

Status: ready-for-agent — filed 2026-10-09 from the issue 498 conversion review (`wf_a128d5c8-b83`, error-path
lens), confirmed against turso_core 0.7.2's `abort`. NEXT: unit 1.
Kind: correctness / the Phase-1 resolver (`crates/store/src/canonical.rs`)
Relates to: 498 (the transaction helper; this is the one hole it cannot close), 448 / 470 / 318 / 351 (the four
lenient probes)

## What is wrong

`resolve_mentions` runs each `WRITE_BATCH` chunk in one transaction (`Db::immediate` since issue 498). Inside it,
four SELECT probes on the writer swallow their error and carry on "leniently":

| Site | Probe | On error |
|---|---|---|
| `altid_alias_bind` (issue 448) | `name_key_is_generic_on` | binds as if the name were not generic |
| `resolve_one_mention` (issue 470) | `name_key_is_generic_on` | binds as if not generic |
| `resolve_one_mention` (issue 318) | genericness probe | binds leniently |
| `resolve_one_mention` (issue 351) | `echo_tier_on` | mints a country-less provisional |

Leniency assumes the failed probe left the transaction alone, and turso 0.7.2 does not guarantee that.
`Program::abort` (`vdbe/mod.rs`, the `_ => match state.auto_txn_cleanup` arm) handles a statement with
`TxnCleanup::None`, which is what a read inside an explicit transaction gets. For any error outside the named
arms (Busy, TxError, Constraint and so on), it runs `rollback_current_txn` when
`!auto_commit && err.is_some()`. **The whole chunk transaction is gone**, and the connection is back in
autocommit.

The resolver then carries on:

- Every later organization, mention and `append_change` write in the chunk commits on its own.
- Some of those rows can name organization ids minted earlier in the chunk, which were just rolled back.
- `finish`'s COMMIT fails ("no transaction is active") and the run aborts. But the autocommitted tail stays
  on disk: a half chunk whose `changes` rows announce organizations that do not exist.

How rare: the probes have never errored in prod that we know of. The error would have to be a runtime one (I/O,
a corrupt page, an internal error), not a prepare error. It needs fixing anyway, because the damage is silent and
the leniency was written on the opposite assumption.

## Fix

A probe's error is lenient only while the caller's transaction survived it. In each of the four `Err` arms, when
the function was entered inside a transaction and the connection is now in autocommit, return the error instead
of carrying on. The run then aborts with the chunk already rolled back by the engine, which is the state
`Db::immediate` would have produced.

## Units

1. **Pin the turso behaviour.** A store test: inside `BEGIN IMMEDIATE`, write a row, run a SELECT that fails at
   runtime, then assert:
   - the connection is in autocommit;
   - the row is gone.
   If turso ever stops doing this, the test says so, and the guard below becomes dead code rather than wrong.
2. **Guard the four arms**, with a test: a resolver chunk whose probe fails at runtime ends with an error and
   leaves no rows from that chunk behind.
