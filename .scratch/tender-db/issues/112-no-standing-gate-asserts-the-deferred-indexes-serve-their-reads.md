# 112 — no standing gate asserts the hot read paths actually use an index

Status: open — filed 2026-08-03 (sdk-vendor), from the `/v1/tenders/{id}` ~2.2s regression
Kind: verification (missing gate)
Blocked by: 110 — DONE (section I landed `a2835b5`); this is next
Relates to: 111 (the app-side detect+repair this gate must verify INDEPENDENTLY), 89, 82/83, 62/60, 107

Scope note: filed as "the deferred indexes serve their reads" (the filename), broadened
2026-08-03 to **any hot read-path query whose plan can silently degrade to a scan**.
The deferred indexes are one cause of that; the turso planner gap below is another, and
the gate that catches one catches the other. The filename is left alone to avoid churn.

## Motivating example — this is not hypothetical, and it refutes the issue's own first draft

Filed on the theory that `tender_version_bid_parties_version` was **missing** on prod.
run-driver then read the live schema from the post-refold snapshot: **the index exists**
(job 534's reindex built it). The unapplied-migration thesis was right about the code and
wrong about the live DB — a code-read cannot see which jobs actually ran.

The real cause of the ~2.2s is a **turso planner gap**: `lots_of` (read.rs:944 → `lots()`
with `Scope::Page { after: 0, limit: MAX_PAGE }`) scans the 13.2M-row `lots` table on every
request, because turso declines to use the existing `UNIQUE(tender_id, lot_key)`
(canonical.rs:113-114) under an `ORDER BY l.id LIMIT` shape.

**The index exists and the planner scans anyway.** That is precisely why this gate asserts
the *plan* and not the *name*: the naive presence-check version of this gate — the obvious
one to write — would have gone green over a 2.2s full scan. Presence-of-name was already
the correlate; this incident is the proof, and it widens the target from "is the index
there?" to "is the read actually served by one?"

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

### 1. Assert the query PLAN of every HOT READ, not the index name

Presence-of-name is a **correlate** — and per the motivating example above it is a
correlate that was *already true* while the site served 2.2s pages. The load-bearing
signal is that the planner actually picks an index for the read:

**Target set = the hot read-path queries**, not just the deferred-index-backed ones.
Each must show `SEARCH … USING INDEX …` and must not show `SCAN`:

| read | table | the plan must not scan |
|---|---|---|
| `lots_of` / `GET /v1/lots?tender=` | `lots` (13.2M) | the turso `ORDER BY l.id LIMIT` gap — **live defect today** |
| `tender_detail` satellites | `tender_version_*` by `(tender_id, seq)` | issue 89's shape |
| the incremental identity probe | `tenders(procedure_key)`, `tenders(source, island_notice_id)` | full scan of 8.1M **per folded Tender** |
| the Phase-1 mention resolver | `organizations(country, kind, identifier)` | full scan of ~30M per new mention |
| the newest-Tenders list | `tenders(current_published_at, id)` | issue 25/82 |

The last three matter disproportionately because they are invisible from the outside:
a scanning identity probe surfaces only as "the daily fold got slow", never as a slow
page. A latency canary on `/v1/tenders/{id}` would not catch them; an EQP assertion does.

For the index-backed reads specifically, the plan check is what the name check was
standing in for:

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

### 1b. The plan must come from TURSO — sqlite3 is the wrong engine (measured)

Found while implementing, and it invalidates the obvious read path. `/v1/sql` rejects
`EXPLAIN` outright (`v1/sql.rs:553`), so the live HTTP path cannot produce a plan at
all. That leaves a snapshot — and **stock SQLite disagrees with turso on exactly the
query that motivated this issue.** On the real `lots` schema, with and without
`ANALYZE`:

```
EXPLAIN QUERY PLAN SELECT l.id, l.tender_id, l.lot_key FROM lots l
  JOIN tenders t ON t.id = l.tender_id WHERE l.tender_id = 42 ORDER BY l.id LIMIT 1000;

|--SEARCH l USING COVERING INDEX sqlite_autoindex_lots_1 (tender_id=?)
```

`SEARCH`, not `SCAN` — **GREEN** — for the read turso scans in prod. A snapshot is the
right *data*; sqlite3 is the wrong *engine*, and a plan verdict from the wrong engine
would have certified the live 2.2s defect as healthy. That is issue 110's mirror one
level deeper: validating turso's behaviour against a different engine's assumptions.

So the plan half **requires** a turso-backed source at the deployed version and reports
`no-input` without one — it must never fall back to sqlite3. Ranked sources:

1. **A turso-linked harness** run over a snapshot — right engine, and independent of
   the app, so it does not become a mirror of the thing it checks.
2. **An app-side diagnostic** running EQP through the server's own pool. For *plans*
   this is arguably the most load-bearing source that exists — it is literally the
   engine serving traffic — and it is metadata-only, so it stays live-safe.
3. Stock sqlite3 — **not evidence** for this defect class. Valid only for the
   index-presence/DDL half, where reading a catalogue is not planning a query.

### 1c. "SEARCH not SCAN" is ALSO a correlate — turso's plan text misleads

Third false-green in this story, found by run-driver. Turso prints the `lots` full walk
as:

```
SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
```

That reads like a point lookup. It is a forward walk of all 13.2M rows. So a
`SEARCH`-vs-`SCAN` assertion — the obvious one, and the one this issue originally
specified — **passes the defect it was built for**, even with the correct engine.

The load-bearing signal is the **access path the plan names**: the target table's line
must name a real INDEX. A rowid / `INTEGER PRIMARY KEY` access on the target is RED
exactly like a SCAN. Where a specific index is the point of the read, the name is
required too, since a different index can still be the wrong path.

Two scoping rules that matter:
- **The rowid rule applies to the target table only.** In the `lots_of` plan,
  `SEARCH t USING INTEGER PRIMARY KEY (rowid=?)` for the joined `tenders` is a correct
  primary-key point lookup and must not be flagged.
- **`lots` is served by an implicit index** (`sqlite_autoindex_lots_1`, from the UNIQUE
  constraint), so the expected-index rule must accept it — a rule that only ever
  accepted explicitly-named indexes would be wrong for this very read.

The running tally of correlates that each looked load-bearing and were not: the index
exists → sqlite3 says SEARCH → turso says SEARCH. Each was one level closer to the
truth and still green over a live 2.2s scan.

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
| **pass** | every hot read's plan is `SEARCH … USING INDEX`; every const entry present with DDL matching cols | — |
| **fail** | a plan says `SCAN`, or an entry is missing / has wrong cols | name the specific read, and the index it should have used |
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
5. **any turso version bump.** The `lots_of` defect is a planner behaviour, not a
   schema fact: the same schema and the same query can change plan under a new engine,
   in either direction. Nothing else in the suite would notice a regression that
   arrives with a dependency upgrade rather than with our own code.

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
- **Run it against prod as it stands today, THROUGH A TURSO PLAN SOURCE → it must go
  RED on `lots_of`**, with `tender_version_bid_parties_version` GREEN. This is the
  sharpest acceptance test available, because it is the live state: the index that is
  present passes, and the scan that no presence-check could see fails. A gate that goes
  fully green against today's prod is measuring the wrong thing and must be rejected.
  **Caveat added after measuring (§1b): this is unachievable with stock sqlite3**, which
  reports SEARCH for that shape. Run under sqlite3 the gate cannot go red here, and a
  green must be read as no-input, not as a pass.

## Status of the implementation

Landed as `canonical-verify/hot_read_plans.sh` (`ec936e6`). Falsified before commit in
every dimension: drop an index → FAIL naming it; same name with wrong columns → FAIL
showing declared vs on-disk; plan says SCAN → FAIL; plan empty/unrecognisable →
no-input; run outside the repo (no `git show`) → refuses rather than assumes.

Two silent-failure traps closed while building it, both of the "passes having checked
nothing" kind this issue is about:
- **`plan_*` scratch indexes are excluded.** `clear_plan` drops them by design, so
  asserting their presence would fail *correctly* — the fastest way to get a gate
  ignored.
- **The const parse avoids gawk's 3-arg `match()`.** The box's `/usr/bin/awk` may be
  mawk, where that is a syntax error — yielding an empty expectation and a section that
  reports "0 missing" having examined nothing. An expectation of fewer than 5 indexes is
  now refused outright as no-input.

Access-path assertion landed in `3f6e2f7` (§1c): rowid-walk on the target → FAIL, wrong
index → FAIL naming both, healthy plan → PASS with the joined rowid lookup correctly not
flagged. Full clean run 27 pass / 0 fail / 0 no-input.

Open: it needs a turso plan source (`TDB_PLAN_CMD`) to produce any plan verdict. Until
one exists, section B is permanently no-input and only the presence half is live.

**Decision on the plan source: option 1, the turso-linked harness**, on the independence
tie-breaker — a gate that borrows the app's own EQP path is closer to grading its own
homework, and the whole point of this issue is that the gate must be able to catch a
defect in the thing it checks. Option 2 (app-side diagnostic through the serving pool)
stays an acceptable fallback if the harness proves heavy, since for *plans* it has the
strongest claim to being load-bearing — it is literally the engine serving traffic.

The contract is deliberately trivial so either can satisfy it: `TDB_PLAN_CMD` reads SQL
on stdin and writes a turso-produced plan on stdout. If run-driver-2's `planlab-lots_of`
experiment already runs through turso, that harness *is* the plan source and wiring it in
is one environment variable.

## Sequencing

110 is **done** — section I landed greenfield in `a2835b5` (it had never existed in the
committed suite; the prior session committed the issue and not the code). This is next.
Then re-run the fixed suite against a snapshot when nothing is serving, as the durable
record.
