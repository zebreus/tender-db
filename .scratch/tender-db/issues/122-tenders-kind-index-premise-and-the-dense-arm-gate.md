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
density grounds. **That premise is false: the planner has no statistics for any read-path table.** It
cannot know that `procedure` is dense and `registration` is rare — it sees one index on the filtered
column and uses it for every value. So the risk does not just remain, it **inverts**: the index is
guaranteed to be used exactly where it is most dangerous.

> **Correction (run-driver, measured 2026-08-04).** I first wrote that "prod has no `sqlite_stat1`".
> That is **false as stated**. `sqlite_stat1` exists on both the serving DB and the snapshot — it holds
> exactly **one** row, for `plan_notice` / `plan_notice_fold`, a projection-internal table. No read-path
> table appears in it: not `tenders`, `tender_versions`, `lots`, `organizations`, `notices`. The
> operative conclusion is unharmed, but the stated fact was wrong and would have mis-specified the bed
> (see the requirements below).
>
> The property is **maintained by construction**, not merely true today: the only `ANALYZE` anywhere in
> the codebase is the table-scoped `ANALYZE plan_notice` at `canonical.rs:1956` (added so Phase-2's
> `next_plan_batch` streams via the fold index instead of sorting the `group_key` tail). It cannot
> broaden. The named way this breaks is a human running a bare `ANALYZE` on prod — which would populate
> every read-path table and silently change the planner underneath every measurement in this issue.

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

**The correction above strengthens this design rather than weakening it.** The stats state of prod turned
out not to be what anyone believed — I asserted "no `sqlite_stat1`", it exists, and it took someone
measuring it to find out. That is the argument for the partial index rather than the plain one: a partial
index the planner **may not use** for the dense value is safe *regardless of what the planner knows*,
while the plain index's safety is a claim about planner behaviour under a particular stats state. Only
one of those two survives being wrong about the stats. Given that we were just wrong about the stats,
prefer the one that does not depend on them.

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

0. **No *read-path* table has a `sqlite_stat1` row** — run-driver's restatement of my requirement, and
   correct where mine was not. I asked for "no `sqlite_stat1` at all", which fails in both directions:
   too strict, because building 30a by running the real projection creates the `plan_notice` row and a
   prod-faithful bed would then be rejected by a check meant to keep it prod-like; and too loose, because
   "no stat1 table" is not what prod looks like, so the bed would differ from prod in a stated-but-untested
   way — the exact class of difference this requirement exists to prevent. The checkable property is a
   one-liner over `sqlite_stat1` for the read-path tables, expected 0. The real hazard behind my wording
   was a bare `ANALYZE` on the fixture, and that is what must never run. Same species as the
   sqlite3-vs-turso trap `hot_read_plans.sh` was built around, one layer up.
1. ~~**`registration` rare AND LATE in id order.**~~ **WRONG — corrected by measurement, see below.**
   The property is not lateness. It is: **fewer than `LIMIT` matching rows remain after the cursor, while
   much of the table is still ahead of it.** The read walks `t.id > ? ORDER BY t.id LIMIT 50` and can only
   stop early when `LIMIT` fills; if it cannot fill, it walks to the end of the table. Late matches are one
   way to cause that. **Exhausted matches are another, and that is what prod actually has.**

   > **Correction (run-driver, Phase A measurement 2026-08-04).** Prod's `registration` is **120 rows in
   > ~8.1M (0.0015 %)**, sitting in id deciles **1–2** — first match at id 1,127,544, about **14 % in**.
   > Rare, and **early**. My "rare AND late" was an invented positional claim, and I asserted it in the
   > same list as requirement (2), which says *derive the distribution from the snapshot, do not invent it —
   > including not by me*. I violated my own requirement one line above writing it.
   >
   > The mechanism is therefore **matches-early-then-exhausted**: the first page fills quickly from the
   > cluster and is *fast*; once the 120 are consumed, every later page walks the remaining ~75–85 % of the
   > table and returns nothing, and the last pages are full scans for zero rows. That fits an 18.7 s
   > *ordinary* read better than lateness does, and it predicts something lateness does not — **the cost
   > depends on cursor position**. Also measured: there is no `contract` kind in prod at all; the dominant
   > value is `procedure`.

   Consequence for the bed: reproduce the **property**, not the position — a `kind` value whose matches run
   out with most of the table still ahead of the cursor.
2. **`kind` distribution derived from the snapshot, not invented** — including by me. A guessed skew is
   the same unstated-assumption-about-the-corpus that made 117's original `?kind=` resolution wrong.
3. **A selective co-filter independent of `kind`** (cpv / nuts / published window) matching few rows
   scattered across the corpus. This is what makes check 1 of the gate possible and is the part an
   ordinary sweep would not include.
4. **Non-empty satellites** — the list read joins `tender_versions` and runs correlated subqueries per
   row; empty satellites flatter exactly the plan under suspicion.
5. **Deep-cursor positions**, not only `after = 0`. **Promoted from nice-to-have to LOAD-BEARING** by the
   correction above: with matches clustered early and then exhausted, `after = 0` is the *cheap* page — the
   pathology is invisible there and only appears past the cluster. A clock that sampled only the first page
   would have reported this read healthy. The anchor must therefore be clocked at a cursor **past the
   exhaustion point**, which is where the 18.7 s lives.

The fixture is anchored to reproduce the known 18.7 s before its other numbers are trusted. **If it
cannot reproduce it, this issue waits** rather than measuring against a bed that lacks the phenomenon.

### The anchor is TWO clauses, because one number cannot catch an inverted geometry

run-driver established from issue 117's own wording — *"sent by ordinary clients with no crafted input"*,
which excludes a hand-built cursor — that the **18.7 s was the FIRST page, `after = 0`**. Phase A's
geometry corroborates independently: the first match sits at id 1,127,544, so page 1 walks ~1.18 M rows of
the driven table before `LIMIT` can even begin filling. So **18.7 s is a floor, not the worst case.**

Page by page, with 120 matches and `LIMIT 50` (worked from Phase A's ids):

| page | rows returned | table rows walked | cost |
|---|---|---|---|
| 1 | 50 | ~1.18 M (id 1 → 50th match) | the measured 18.7 s |
| 2 | 50 | the cluster span only | **fast** |
| 3 | 20 | ~6.2 M — cannot fill, so walks to the end | **worst** |
| 4+ | 0 | ~6.2 M for nothing | worst, repeatedly |

Note it is **page 3**, not page 2, that first cannot fill — pages 1 and 2 both fill from the 120-row
cluster. The bed therefore needs enough matches that pagination *reaches* an unfillable page, and the
clock must sample that page specifically.

So the anchor is:

1. page 1 at `after = 0` reproduces ~18.7 s, **and**
2. the post-exhaustion page is **strictly worse** than page 1.

Clause 2 is the discriminating one and clause 1 alone is not sufficient: a bed that hits 18.7 s on page 1
while showing later pages *cheaper* has the geometry inverted and is wrong however well it matches the
headline. A single-number anchor cannot detect that — which is the same shape as every other check
corrected today, one level up: the number agreed, and the thing behind it did not.

> **The anchor check itself needed correcting.** As first written it asserted that the rarest kind first
> matches *past 50 % of the id range* — my "late" prose turned into a gate. Against a prod-faithful bed
> that check **rejects the correct fixture**, because prod's rare kind matches at ~14 %. run-driver caught
> it before building to it. Corrected anchor: *the rare kind's matches are exhausted with most of the table
> still ahead, and the clock samples a cursor past the exhaustion point.*
>
> Worth noting what this was: a hypothesis of mine, promoted to an assertion in a gate, which would then
> have enforced my error against the measurement that disproved it. A gate encoding an unmeasured belief
> does not just fail to catch the problem — it actively rejects the truth.

## Note on `#15 ≠ #16`

Different columns and different tables: task #16 was `tender_version_lots.kind` (`?kind=Lot`), already
shipped and live. This is `tenders.kind`. The names collide; the reads do not.
