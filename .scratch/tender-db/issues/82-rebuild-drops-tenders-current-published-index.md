# 82 — rebuild drops `tenders_current_published` and never rebuilds it → unindexed list + failing ~49min boot

Status: open — ROOT-CAUSED 2026-08-01 (run-driver, strace-level). Deployed rev 4272266. Workaround applied (boot rebuilds it once, after issue 83's TMPDIR fix); needs the permanent code fix below.
Kind: correctness / performance
Blocked by: —
Relates to: 63 (drop_and_recreate teardown, commit 1f6ab90), 62 (bucketed fold / end index builds), 66

## Symptom (post-recovery-rebuild, 2026-08-01)

After the full recovery rebuild (`project rebuild=true clear_changes`, job 1) completed
successfully (8.1M tenders, snapshot integrity ok), `/api/tenders` (the newest-first list)
**hung / full-table-scanned** — 40s+ timeouts, while `/api/tenders/:id` point lookups stayed
fast (~50ms). strace showed leaked 4 KB-page sequential `pread64` scans marching across the
~300 GB region with no client attached (HTTP clients had disconnected at timeout but the
uncancelled queries kept scanning — a secondary leak worth its own hardening).

## Root cause

`crates/store/src/lib.rs::migrate()` creates:

```sql
CREATE INDEX IF NOT EXISTS tenders_current_published ON tenders(current_published_at, id)
```

which backs `/api/tenders`' `ORDER BY current_published_at DESC` list query. The rebuild's
`reset_tender_layer` / `clear_canonical` **`drop_and_recreate`** (commit 1f6ab90, the deadlock
fix) drops+recreates `tenders` bare, so the index is gone. The projection's end index-build set
(`build_tender_indexes`) rebuilds the org/tender-satellite/changes indexes but **NOT**
`tenders_current_published` (it's a `migrate()`-only index, not in the projection's list). So a
completed rebuild leaves the list endpoint unindexed → full scan.

On the next boot, `migrate()`'s `CREATE INDEX IF NOT EXISTS` finally builds it — this is the
"~49 min stripped-index boot": 6.3 GB read, ~28 KB written, random B-tree access, external-sort
spill. (That boot was itself failing until issue 83's TMPDIR fix — see [[83-service-tmpdir-tmpfs-sort-spill]].)

## Fix (permanent, choose one)

1. **Add `tenders_current_published` to the projection's end index-build set** (`build_tender_indexes`),
   so a completed rebuild leaves the tender layer fully indexed and boot has nothing to build. (Preferred —
   keeps all tender-serving indexes owned by one place.) OR
2. Make `drop_and_recreate` **recapture and rebuild every index** that existed on the table (not just the
   table DDL) — but the migrate()-created index isn't present at rebuild time (the table was dropped bare),
   so this only works if the index list is sourced from `migrate()`'s canonical set, not from the live
   sqlite_master at drop time. Option 1 is cleaner.

## Validation

After the fix, a `rebuild=true` completes with `tenders_current_published` present (PRAGMA index_list
tenders), `/api/tenders` (list) returns fast (indexed, ms), and the subsequent boot does NOT spend ~49min
building an index. Add a test asserting the index exists post-`reset_tender_layer`+`build_tender_indexes`.

## Secondary hardening (separate, smaller)

Uncancelled query leak: a disconnected HTTP client's `/api/tenders` query kept running (4 leaked full
scans thrashing the disk in lockstep). List queries should be cancelled on client disconnect and/or bounded
by a statement timeout, so a slow endpoint can't accumulate concurrent full scans.
