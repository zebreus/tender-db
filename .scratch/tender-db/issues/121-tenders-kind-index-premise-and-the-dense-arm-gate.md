# 121 — `tenders(kind, id)`: the premise for the index is false, and the dense arm is the gate

Status: DESIGN — filed 2026-08-04 (proj-fix) for task #15. **Do not build the index on the stated
premise.** Needs the dense-arm measurement below before any DDL.
Kind: read latency / regression risk
Blocked by: — (the measurement is local; no prod contact)
Relates to: 117 (the audit and its `?kind=` correction — that file has a single editor, so this design
lives here rather than in it), 111 (deferred-index builder), 112 (the standing plan gate), 120

## Where this came from

Task #15 followed issue 117's correction: `?kind=registration` measured **18.7 s on prod**, an ordinary
documented value, and `t.kind` is a column of the driven table, so it is the one Class B member an index
can fix. The task was written as "add `tenders(kind, id)`".

The reason given for it being safe was that the planner would decline the index for the dense value on
density grounds. **That premise is false: prod has no `sqlite_stat1`.** With no statistics the planner
cannot know that `procedure` is dense and `registration` is rare — it sees one index on the filtered
column and uses it for every value. So the risk does not just remain, it **inverts**: the index is
guaranteed to be used exactly where it is most dangerous.

## What the danger actually is (from the statement, not from a hunch)

The list read (`store/src/read.rs`) paginates on the primary key:

```sql
… FROM tenders t JOIN tender_versions v ON … WHERE 1 = 1
  AND t.kind = ?            -- the filter
  … version_predicates …    -- cpv / country / published window / etc.
  AND t.id > ? ORDER BY t.id LIMIT ?
```

`ORDER BY t.id` is the important part. A `(kind, id)` index seeks to `(kind, after)` and walks in id
order, so the ordering is satisfied and the walk **terminates at `LIMIT`** — for a sparse value *and* a
dense one, as long as `kind` is the only filter. That case is not the hazard.

The hazard is `?kind=<dense>` **plus** a selective `version_predicates` co-filter. Then:

- **without** the index: a sequential walk of `tenders` in rowid order, stopping when `LIMIT` fills;
- **with** the index: a walk of the ~7.9 M `procedure` index entries, each needing a **random main-table
  indirection** to evaluate the co-filter, until `LIMIT` fills or the corpus ends.

Random-per-row indirection replacing a sequential scan is precisely the regression class the `lots`
fix (task #16) was careful to avoid. So: **the dense arm is the gate for shipping this index, not the
confirmation.** `?kind=registration` getting fast proves nothing on its own — a fix that makes the rare
value fast and the common value pathological is a net loss, and `procedure` is the common one.

## Proposed design — make the trap unreachable instead of hoping the planner avoids it

A **partial** index:

```sql
CREATE INDEX tenders_kind_sparse ON tenders(kind, id) WHERE kind <> 'procedure';
```

The planner may only use a partial index when it can prove the query implies the WHERE clause, so
`?kind=procedure` **cannot** use it, with no reliance on statistics that do not exist. `?kind=registration`
(and any future rare value) gets the seek. The dense arm keeps exactly today's plan, so its worst case
cannot regress.

The honest cost: this writes a corpus fact — *`procedure` is the dominant value* — into the schema, which
is the same species of unstated assumption that issue 117's correction was about. The difference is the
failure mode. If `procedure` stopped being dominant, this index simply stops helping the value that no
longer needs it; nothing regresses, and no read gets slower than it is today. That is a benign
degradation, not a latent trap, and the assumption must be written at the definition rather than left to
be inferred.

Rejected alternative: an app-side short-circuit like `?country=`'s. That one is sound because a filter
matching ~everything can be *dropped* without changing the result. `kind` has three or more values, so
dropping the predicate changes the answer. Not available here.

## What must be measured before any DDL (the gate)

Local plan-lab, at prod-like row counts and prod-like value skew — no prod contact, no deploy:

1. **Dense arm, co-filtered — the gate.** `?kind=procedure` + a selective `version_predicates` co-filter,
   plan and time, three ways: no index / plain `(kind, id)` / the partial index. The plain index is
   expected to regress here; **if it does not, this whole design is unnecessary and should be dropped in
   favour of the plain index** — that outcome is a result, not a failure.
2. **Sparse arm.** `?kind=registration` must go from a full walk to a seek under the partial index.
3. **Dense arm, uncofiltered.** `?kind=procedure` alone must keep its current plan and stay fast under
   the partial index (this is the "no regression" check, and it is the cheap one — do not mistake it for
   check 1).
4. **The proof that the exclusion holds:** `EXPLAIN QUERY PLAN` for `?kind=procedure` must show the
   partial index is **not** used. That assertion belongs in the checked-set (issue 112) so it stays true,
   not just in a one-off run.

Ship only if 1 shows the regression, 2 shows the win, 3 shows no change, and 4 holds. Any other
combination means the premise moved again and the design should be re-derived rather than patched.

## Note on `#15 ≠ #16`

Different columns and different tables: task #16 was `tender_version_lots.kind` (`?kind=Lot`), already
shipped and live. This is `tenders.kind`. The names collide; the reads do not.
