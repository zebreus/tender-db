# 549 — `refold-fields`' carrier sweep holds the single writer for ~9 minutes of read-only work, even on a sizing run that writes nothing

Status: needs-triage — filed 2026-10-15 from the hourly audit (`/metrics` `tender_db_writer_longest_wait_seconds`
507). Next: unit 1, list every read-only accessor that runs on the writer for more than a second, then move them to
the reader pool.
Kind: operations (writer contention)
Relates to:
- 20 (read-only accessors use a pooled reader, "never the writer");
- 241 (writer wait is measured, and a slow wait is journalled with its caller);
- 237 (`notice_ids_with_section_kind`, the indexed sibling);
- 532 and 534 (the two sizings that showed it).

## Verify

    grep -n "pub async fn notice_ids_carrying_fields" -A14 crates/store/src/canonical.rs | grep -c "self.conn().await"

- **done**: `0`. The sweep borrows a reader, and a sizing `refold-fields` run (`expect` far off) journals no
  `writer acquired after a … s wait` line.
- **open (2026-10-15)**: `1`.

## What was found

`/metrics` showed `tender_db_writer_longest_wait_seconds 507.13`. The journal timestamps are CEST:

    15:32:34 [store] writer acquired after a 503.0 s wait by crates/store/src/canonical.rs:40687 (1 caller(s) still queued)
    15:46:01 [store] writer acquired after a 507.1 s wait by crates/store/src/canonical.rs:40687 (1 caller(s) still queued)

The waiter is coverage's `layer_presence` heartbeat (`UPDATE layer_presence SET observed_at`). The holder is
`refold-fields`' carrier sweep, `Db::notice_ids_carrying_fields` (canonical.rs ~13592). It opens `self.conn()`,
the writer, and walks `notice_codes` in 500,000-id windows (`SELECT DISTINCT notice_id … WHERE field_id IN (…)`,
since `field_id` is unindexed) for 8–9 minutes:
- once for job 2519, issue 532's sizing (`expect: 1`, aborted, "nothing was written");
- once for job 2521, the wet run.

The supervisor's own phase text calls the sweep "read-only". It needs no write lock: the re-queue and the stamp that
follow it take the writer separately.

Consequences:
- Any other write queues behind a pure count for up to ~9 minutes: the coverage heartbeat, admin writes, the daily
  `probe`/`process`/`project` chain if the two overlap, and webhook/delivery bookkeeping.
- A sizing run, which runbooks recommend precisely because it is harmless, is the case that blocks writers longest
  for no write at all. 534's sizing (job 2494, 531 s) did the same.

## Units

1. **Audit (S).** List every `Db` method whose doc or behaviour is read-only but which opens `self.conn()`, ranked
   by how long it can run. Candidates:
   - `notice_ids_carrying_fields`;
   - `notice_ids_with_section_kind`, which also opens `self.conn()` (canonical.rs ~13656), though it is an index
     read and short;
   - the census-style walks.

   The writer-wait journal lines (241) give the prod evidence.
2. **Move them (S–M).** Use `self.reader()`. First check:
   - the read pool's per-statement deadline, if any: a 500k-id window took ~36 s on prod, so the window may need to
     shrink, or the job path may need a reader without the API deadline;
   - the WAL-gate interplay of a long reader during a heavy write.

   Add a test, or extend `the_refresh_plans_candidate_reads_seek`'s pattern, asserting that the sweep does not
   take the writer. For example, hold the writer in the test and expect the sweep to finish.
