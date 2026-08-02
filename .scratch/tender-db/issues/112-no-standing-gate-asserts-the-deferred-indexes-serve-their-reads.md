# 112 — no standing gate asserts the deferred indexes actually serve their reads

Status: open — filed 2026-08-03 (sdk-vendor), from the `/v1/tenders/{id}` ~2.2s regression
Kind: verification (missing gate)
Blocked by: 110 — land the section-I artifact fix first, then implement this
Relates to: 111 (the app-side detect+repair this gate must verify INDEPENDENTLY), 89, 82/83, 62/60, 107

## What this is, and how it differs from 111

111 is the **fix**: the app learns to detect missing deferred indexes at boot and
enqueue a `reindex` to repair them. proj-fix owns it.

112 is the **gate**: the verify suite asserts, from outside the app, that the live
layer's deferred indexes are present *and are actually chosen by the reads they exist
to serve*. Two different obligations. 111 without 112 means the only thing watching
the indexes is the same component that manages them.

**The load-bearing requirement, and the reason this issue exists separately:** this
gate must NOT be implemented by calling 111's `missing_deferred_indexes()` and
asserting it returns `[]`. That is the app grading its own homework — a mirror, and
it would pass cleanly if 111's detector itself has a bug (wrong name list, a typo in
the `IN (…)` clause, a query that silently returns no rows). The gate has to derive
its expectation and its observation independently, so that it can catch a defect in
the detector. Same lesson as the DE-1.x fixture that passed against a stub authored
on both sides, and as 110.

## Why nothing caught the ~2.2s regression

Every existing gate counts rows (see 109). The layer was *correct* the whole time —
right rows, right values, every DE-1.x number reconciled. What was wrong was the
**access path**, which no gate looks at. A gate suite that only ever asks "are the
facts right?" cannot see a full scan, because a full scan returns the right answer.

The suite also had no notion that a code-declared index is not a live index
(111's root cause). Verification never reconciled the deployed build's
`DEFERRED_TENDER_INDEXES` against `sqlite_master`.

## Design

### 1. Assert the query PLAN, not the index name

Presence-of-name is a **correlate**. The load-bearing signal is that the planner
picks the index for the read it was added for:

```sql
EXPLAIN QUERY PLAN
SELECT … FROM tender_version_bid_parties WHERE tender_id = ?1 AND seq = ?2;
-- must contain: USING INDEX tender_version_bid_parties_version
-- must NOT contain: SCAN
```

This catches a case pure presence waves through: an index that exists under the
right name but was created from a **stale definition** (right name, wrong columns —
e.g. left over from a prior schema) serves the read no better than nothing, and a
`name IN (…)` check passes it. Belt-and-braces: also compare each entry's
`sqlite_master.sql` DDL against the const's declared `cols`.

Cover all twelve (10 `DEFERRED_TENDER_INDEXES` + the org pair). The EQP assertion
matters most for the four 111 identifies as having no schema-batch/`migrate()`
fallback — `tender_version_bid_parties_version`, `tenders_procedure_key`,
`tenders_island`, `organizations_identity` — since those are the ones a deploy
cannot create. Assert the identity-probe reads too, not just `tender_detail`: a
missing `tenders_procedure_key` full-scans 8.1M rows *per folded Tender* and would
surface only as "the daily fold got slow" (111's discriminator table).

### 2. Derive the expected set from the DEPLOYED BUILD — never a hand-copied list

**This is the whole game.** A shell gate with ten literal index names goes green the
day an eleventh const entry lands — the exact failure that caused this incident,
reproduced one level up inside its own detector. Acceptance requires that adding a
const entry with no other change makes the gate go RED on an un-reindexed DB.

- **Interim (no code change):** parse the const block out of
  `git show <deployed-sha>:crates/store/src/canonical.rs`. Expectation is pinned to
  the deployed commit, not to the gate author's typing.
- **End state:** a small diagnostic that reconciles the binary's own const against
  `sqlite_master` and serves the result. The app knows both sides, so it "knows its
  own inputs". Note this is 111's `missing_deferred_indexes()` — and per the header,
  when the gate consumes it, it must still corroborate independently (EQP is the
  independent observation; the const parse is the independent expectation).

The deployed sha must be *established*, not assumed — reuse 107's freshness-witness
discipline. If the gate cannot prove which build is serving, that is `no-input`,
not `pass`.

### 3. Three states, never two

| state | meaning | alarm |
|---|---|---|
| **pass** | every const entry present, DDL matches cols, EQP uses it | — |
| **fail** | an entry missing / wrong cols / plan says `SCAN` | name the specific index and the read it starves |
| **no-input** | `sqlite_master` unreadable, deployed sha unresolvable, EQP unavailable | distinct alarm — MUST NOT silently pass |

The no-input state is the one 110 is about. A gate that cannot tell "verified good"
from "could not verify" is not evidence.

## Why this can be a STANDING check (not snapshot-only)

It is **metadata-only**: `sqlite_master` is a tiny table and `EXPLAIN QUERY PLAN`
compiles without executing — zero data-page I/O, no competition with live traffic.
Unlike every other gate in the suite it is safe against the live DB while it serves.
That property is what makes it worth building: it can run on every trigger below
rather than waiting for a quiet window and a snapshot.

## Triggers

Run after any of:

1. a full rebuild (`rebuild=true`)
2. a `Refold` / incremental fold
3. a `reindex` op (to confirm it actually discharged the obligation)
4. **any deploy whose `DEFERRED_TENDER_INDEXES` differs from the previously deployed
   build's** — the trigger that would have caught this incident on Aug 1, and the one
   a fold-side-only fix misses entirely. This is the deploy/migration path, which is
   where 111 correctly places the root cause.

## Note for 111's implementation (boot-time caveat)

Flagged so it is not lost: the boot-time detection must **detect loudly, build only
on the explicit op** — surface the const-vs-`sqlite_master` mismatch to
`/health`-degraded or an admin banner and enqueue the durable job, but never build an
index inline on the boot path. Building at boot is precisely the multi-hour/hanging
boot of issues 82/83. 111's step 2 (enqueue as a background durable job, boot never
blocks) is the correct shape; this note exists so a later simplification does not
"tidy" it into a synchronous boot build.

## Acceptance

- Strip one deferred index on a scratch DB → gate goes RED and names that index.
- Restore it → GREEN.
- Add an 11th entry to the const, deploy nothing → gate goes RED on the live-shaped
  DB (proves the expectation is derived, not hand-copied).
- Replace an index with a same-name/wrong-columns definition → gate goes RED via
  EQP/DDL (proves presence-of-name is not what is being measured).
- Point the gate at an unreachable DB / unresolvable sha → `no-input`, distinct from
  both pass and fail.
- Break 111's `missing_deferred_indexes()` (e.g. drop a name from its list) → gate
  still goes RED (proves independence from the detector it verifies).

## Sequencing

110 first (its section-I verdict is known-invalid — wrong artifact), then this, then
re-run the fixed suite against a snapshot when nothing is serving as the durable
record.
