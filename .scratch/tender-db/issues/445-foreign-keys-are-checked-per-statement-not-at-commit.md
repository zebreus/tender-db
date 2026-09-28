# 445 — the re-parse comments say foreign keys are checked at COMMIT; turso checks them per statement

Status: done — BUILT 2026-09-28 (see the foot; gate below). Was: ready-for-agent — filed 2026-09-28 from issue 442 step
2's adversarial review.
Kind: code correctness (comments that mislead the next change), small
Relates to: 442 (the review that found it), 441 (the mention FK proof), 352 (a bracket justified by a similar
misreading of turso's pragma)

## What is wrong

`crates/store/src/lib.rs` around lines 1689–1693 and 2429–2434 (the re-parse / reclaim paths) say foreign keys are
"checked once at COMMIT" and carry an `if let Err` on the COMMIT for that case. turso_core 0.7.2 enforces an
immediate (non-DEFERRABLE) foreign key per statement, and the schema declares no DEFERRABLE key. So a violation
surfaces at the statement, and the COMMIT-time branch never fires.

The danger is a future change: someone trusting the comment writes a temporarily inconsistent sequence inside one
transaction, expecting COMMIT to judge it, and gets a failure mid-transaction instead.

## What to do

1. Read both sites against `translate/fkeys.rs` and the delete/insert translation (immediate keys: `FkCheck`
   per statement; the deferred counters only for DEFERRABLE keys).
2. Correct the comments. If the COMMIT-error branch is truly unreachable for FK reasons, say what it still guards
   (I/O, busy) or remove it.
3. Add a store test that pins the per-statement behaviour: inside `BEGIN`, a child insert naming a missing parent
   errors at the INSERT, not the COMMIT.

## Verify

    cargo test -p store <the new test name>

## Done 2026-09-28

The finding was sharper than filed. The comments said "checked at COMMIT" because both sites ran
`PRAGMA defer_foreign_keys = ON`: `reparse_notice` (issue 247) and the moved-identity adoption in
`record_notice_tx`. turso 0.7.2 has no such pragma. `translate/pragma.rs` returns `Ok(())` for any name
`PragmaName::from_str` rejects ("SQLite silently ignores unknown PRAGMA names"). So both statements were no-ops, and
their `if let Err` "unavailable" fallbacks could never fire. Checks were always per statement, which is why issue 247's
~10 s per mention delete lasted until issue 441's exact-shape index.

- **Deleted** both pragma statements and their dead fallbacks. The comments now say what happens: per-statement
  checks, with the proof a seek since 441. The line-2290 note records that the pragma never did anything.
- **Test:** `foreign_keys_are_checked_per_statement_and_the_defer_pragma_is_ignored` (`mention_fk_probe.rs`). With
  the pragma set, inside `BEGIN IMMEDIATE`, an `organization_names` insert naming a missing org fails at the INSERT. If
  a turso upgrade starts honouring the pragma, the test fails and the reasoning gets a fresh look.
- No behaviour change: the statements removed did nothing.
