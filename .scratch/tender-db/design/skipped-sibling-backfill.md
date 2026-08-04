# Backfill: flag the ~593k skipped language siblings so the outstanding count stops lying

Status: DESIGN — the RUN is not mine to authorise. Team-lead is taking the run decision to Lennart, since
a ~593k-row user-facing prod data write is past the reversible-swap envelope and into "irreversible data
operation → fresh human input". **Not urgent**: the ledger card is honest today (it states that the count
overstates), so asking costs nothing.
Owner: proj-fix · Relates to: 84 (the finding), 76 (the reprocess mechanism), 40 (the ledger), 87,
ADR-0009, [[resolved-categories-ledger]], [[prod-disk-constraint]]

## What is being corrected

Issue 84 is settled: the ~593k held `unparsable-xml` / `XML with DTD detected` rows are the non-English
duplicate siblings of ~27k English notices that are themselves present and parsed (verified 5,000/5,000
per-row, two snapshots 29 h apart). They are duplicates already ingested, and **no reprocess can move
them** — dispatch skips a non-English sibling by the one-notice-per-language policy, so it yields no
record to reclaim.

The dashboard's *description* now says this. Its *count* still reports them as outstanding work. This
design closes that, **by representing the outcome** rather than subtracting a magic number.

## Why not `reprocessed_at`

The obvious shortcut — set `reprocessed_at` on the 593k so they drop out of "outstanding" — is wrong and
would be a second lie replacing the first. `quarantine_resolution` reads `reprocessed_at IS NOT NULL` as
**reclaimed**, so this would inflate the reclaimed count by 593k notices that were never ingested. The
honest model needs a third state, because there are three:

| state | meaning |
|---|---|
| outstanding | still held, might yet be reclaimable |
| reclaimed | re-parsed and written into the corpus (`reprocessed_at`) |
| **skipped-as-duplicate** | re-examined, and correctly not ingested — the original is already in |

Proposed: `quarantine.skipped_at INTEGER` (+ `skipped_reason TEXT`, e.g. `internal-ojs-non-english`).
`quarantine_resolution` then reports three buckets and the dashboard shows the third rather than hiding it
inside either of the other two. **A user should be able to see that ~593k rows were examined and
deliberately not ingested** — that is a more honest surface than silently shrinking the count.

## Scope: flag only what is individually confirmed

The per-row predicate, not the aggregate. A row is flagged only if **all** hold:

1. `reason = 'unparsable-xml'` and `detail = 'XML with DTD detected'`,
2. `reprocessed_at IS NULL` and `skipped_at IS NULL`,
3. its fetch is `source='ted'`, `kind='monthly'`, `period LIKE '2008%'`,
4. its member-file language extension is **not** `en`,
5. **its English sibling exists and is `parsed`** — the `<doc>-<year>` publication id, looked up with
   `source` bound so the `UNIQUE(source, publication_id, content_hash)` index seeks (see issue 84's
   section-B hang: without `source` this is a 14.2M-row scan *per row*, which at 593k rows would never
   finish).

Condition 5 is the anti-overreach guard and the reason this is safe: **it makes the operation
self-verifying per row.** A row whose original is missing is exactly a row we must NOT flag — that would
be real data loss being marked as a duplicate, which is the one outcome worse than an overstated count.
Rows failing 5 stay outstanding and are reported.

## Execution envelope

- **Batched**, ~5k rows per transaction by `quarantine.id`, TRUNCATE checkpoint between batches. Not one
  593k-row statement: turso writes a WAL frame per row and a single statement cannot be checkpointed
  mid-way, which is the mechanism that produced the 127 GB WAL and the OOM during the recovery. Small
  batches keep each checkpoint small — and per [[prod-disk-constraint]], live index-build checkpoints
  spike latency, so small is also what keeps the service quiet.
- **Idempotent and resumable** — `WHERE skipped_at IS NULL` means a re-run does the remainder and a crash
  costs one batch. Records a durable resume cursor, the `run_process` pattern.
- **Dry-run first**, and the dry-run is the decision input: counts what it *would* flag, broken down by
  language, plus the count failing condition 5. **Expected: ~593,010 flagged, 0 failing condition 5.** A
  material count failing 5 means issue 84's conclusion does not hold for that subset and the run stops.
- **Reversible** — one column, one scoped `UPDATE … SET skipped_at = NULL`. The job records its id and
  timestamp so the exact set it touched is identifiable afterwards, not merely re-derivable.
- **Through the supervisor**, not a scratch CLI (per [[no-dev-shortcuts-in-prod]]): a narrow admin op, so
  it serialises with ingestion, lands in `job_log`, is cancellable, and counts as a `heavy_write`.
- **Low-traffic window**, and it is a write, so it is prod-affecting by any reading of the boundary rule.

## The permanent half

The backfill is one-time catch-up. Going forward, **the skip must be recorded when it happens**:
`reclaim_package` sees `Disposition::Skipped` and should flag the row then, not merely count it. My
`skipped_by_policy` counter (`6409f56`) is only the reporting half — it made the outcome *visible*, it did
not make it *durable*. Without the permanent half, every future reprocess of a language-fanned bucket
re-creates the same overstatement, and this backfill would need running again.

Order matters: land the permanent half first, then backfill. Reversed, any reprocess run between the two
re-introduces rows the backfill just cleared.

## What is deliberately not decided here

Whether to run it at all, and when. That is Lennart's call via team-lead. This design exists so the ask
can be made against something concrete rather than a sentence.
