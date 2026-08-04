# 122 — `tenders(kind, id)`: the premise for the index is false, and the dense arm is the gate

> **Renumbered 121 → 122.** sdk-vendor's standing-gate issue took 121 (`c02fa0d`, 11:29:34); this landed
> 33 s later (`b82a70d`, 11:30:07) and gives up the number. Commits `b82a70d` and `fc984cb`, and my
> messages to team-lead and run-driver before this note, all say "issue 121" and mean THIS file — read
> those as 122. Two issues sharing a number is the kind of quiet ambiguity that costs someone an hour
> later; the tracker has no allocator, so a collision between agents working in parallel is a
> when-not-if, and the fix is cheap only while both are fresh.

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

## The bed: run-driver's 30a fixture, and what it must carry

Team-lead's call (2026-08-04): this rides on run-driver's **30a** general read fixture rather than a
second bed. The existing bed cannot measure it at all — `tenders.kind` is single-valued there
(`contract` on all 4.26M rows), so `?kind=registration` returns empty in milliseconds and would
**false-green the exact read being redesigned**. Requirements sent to run-driver, in priority order:

0. **No `sqlite_stat1`.** Load-bearing above everything else here. Prod has none, and that is the whole
   premise: with no statistics the planner cannot know which value is dense, so it uses the index for
   every value. A fixture built with `ANALYZE` has a planner strictly smarter than prod's, every plan
   measured is one prod will never produce, and a green would license shipping an index whose safety
   argument was tested against a planner we do not have. Same species as the sqlite3-vs-turso trap
   `hot_read_plans.sh` was built around, one layer up.
1. **`registration` rare AND LATE in id order.** Rarity alone does not reproduce 18.7 s. Because the read
   walks `t.id > ? ORDER BY t.id LIMIT ?`, the pathology is that matches sit late enough that the walk
   nearly completes before `LIMIT` fills. Sprinkle them uniformly and the row counts still look right
   while the phenomenon disappears.
2. **`kind` distribution derived from the snapshot, not invented** — including by me. A guessed skew is
   the same unstated-assumption-about-the-corpus that made 117's original `?kind=` resolution wrong.
3. **A selective co-filter independent of `kind`** (cpv / nuts / published window) matching few rows
   scattered across the corpus. This is what makes check 1 of the gate possible and is the part an
   ordinary sweep would not include.
4. **Non-empty satellites** — the list read joins `tender_versions` and runs correlated subqueries per
   row; empty satellites flatter exactly the plan under suspicion.
5. **Deep-cursor positions**, not only `after = 0`.

The fixture is anchored to reproduce the known 18.7 s before its other numbers are trusted. **If it
cannot reproduce it, this issue waits** rather than measuring against a bed that lacks the phenomenon.

## Note on `#15 ≠ #16`

Different columns and different tables: task #16 was `tender_version_lots.kind` (`?kind=Lot`), already
shipped and live. This is `tenders.kind`. The names collide; the reads do not.
