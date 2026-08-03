# 112 — no standing gate asserts the hot read paths actually use an index

Status: DONE — the gate exists, ran on the box, and confirmed the `1830d50` fix
with a control that held. Filed 2026-08-03 (sdk-vendor), from the `/v1/tenders/{id}` ~2.2s regression
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
on stdin and writes a turso-produced plan on stdout.

**Resolved — wired, no build needed** (`4c6297e`). run-driver-2 left a turso probe on the
box at `/opt/tender-db/turso-bench/plan` (pinned `turso ="=0.7.0"`), invoked
`plan <db> eqp <sqlfile>`. It takes a *file*, not stdin, so the gate carries a small
adapter: set `TDB_PLAN_BIN` + `TDB_PLAN_DB` and section B runs against the right engine
at the deployed version, independent of the app. The app-side EQP diagnostic remains the
better *standing* answer long-term — it survives the probe binary being cleaned up and is
the actual serving engine — but it is a follow-up, not a blocker.

### The stats precondition — enforced, not documented

These schema-only plans are representative **only** while `sqlite_stat1` carries no rows
for the tables under test. Run `ANALYZE` on prod and the planner may choose differently,
at which point every plan this gate produces silently stops describing production —
another input the gate would not know it had lost.

So it is a precondition with teeth: the gate inspects `TDB_PLAN_DB` and reports
**no-input, never a verdict**, if stat rows exist, if the DB is unreadable, or if
`TDB_PLAN_DB` was not given at all. That last case is the subtle one — a plan source
whose input DB is unidentified cannot have its stats state established, so `TDB_PLAN_DB`
is required *even with a custom `TDB_PLAN_CMD`*. A check is only as trustworthy as its
knowledge of its own inputs.

### DONE — B1 synced to the deployed builder, and the swap forced a new control

Issue 114's point 1, applied to this gate itself. Section B's statements were
hand-written to match the shape `read.rs` emits; they were **not** extracted from the
builder. So they can drift: change the query in `read.rs` and the string in the gate
keeps planning the *old* shape — green, while the read that actually runs regresses.
That is the artifact-vs-proxy error of 110 and 102, and it would be a fourth false
green after "the index exists / sqlite3 says SEARCH / turso says SEARCH".

**Closed for B1.** The deployed shape is the row-value cursor (`1830d50`). Its SQL was
*extracted* from the builder, not retyped: at a worktree on `1830d50`, an env-gated
`eprintln!` of `self.sql`/`self.params` inside `read.rs`'s `Query::rows` (the single
choke point every read passes through), with a test that calls `lots_of(&conn, 424242)`.
The only edits applied to that output are newline collapse (the gate's table is
newline-delimited) and substituting each `?` with the value **the builder itself bound**
in the same dump — `[424242, 424242, 0, 1000]`, i.e. tender, tender, `after = 0`,
`limit = MAX_PAGE`. No token was rewritten. Both substitutions were round-tripped back
through the gate's own parser and compared byte-for-byte against the dump.

**B1b added.** `lots_of` only ever asks `after = 0`, but the public
`/v1/lots?tender=&after=N` runs the identical builder on a real cursor page, and
`1830d50` rejected an `after == 0`-only fix precisely because that variant stayed at a
measured 2237ms. A first-page-only assertion would certify a half-fix as whole, so B1b
is the same extracted text with the cursor moved off the first page.

**B2–B6 are still paraphrases** and are now labelled as such in the script rather than
left implicit. Extending the extraction to them is 114's point 1.

#### The swap invalidated the red→green comparison — hence section C

Replacing B1's statement quietly destroyed the thing the post-fix run was supposed to
prove. The recorded **RED** was produced by the old paraphrase; a post-fix **GREEN**
would be produced by different text. Red→green across two different probes is not a
controlled result — the green could mean "the fix works" or "the new statement is one
this planner happens to like". Fixing the drift would have created a fresh false green,
in the very act of removing one.

So the gate now carries a **negative control (section C)**: the *pre-fix* builder's own
SQL, extracted the same way from `1830d50^`, planned in the same run, on the same DB,
through the same probe, and required to still come back a rowid walk. The two extracted
texts were diffed and differ in exactly one place —

```
pre :  AND l.tender_id = ? AND l.id > ?                    ORDER BY l.id LIMIT ?
post:  AND l.tender_id = ? AND (l.tender_id, l.id) > (?, ?) ORDER BY l.id LIMIT ?
```

— so with C RED and B1 GREEN in one run, the cursor predicate is the only variable and
the fix is the only available explanation. If C ever goes **green**, the probe has
stopped discriminating (wrong DB, wrong turso, `ANALYZE` stats appeared) and the run
exits `VOID`: every section-B verdict is uninterpretable, and a B1 green is the fourth
false green rather than the fix.

Falsified in both directions before use, with a stub plan source:

| stub says | B1/B1b | C1 | verdict |
|---|---|---|---|
| rowid walk for everything | FAIL | PASS | gate red, control sound |
| index seek for everything, *including the known-bad shape* | PASS | **FAIL** | **VOID** — greens refused |
| no plan source | no-input | no-input | INCOMPLETE, "section B is UNCONTROLLED" |

The middle row is the one that matters: it is the run where the gate reports the good
news it was built to report, and refuses to let it stand.

One incidental find while wiring C, worth keeping because it is the same failure class:
the control SQL was first embedded as `CONTROL_SQL='…'`. The statement contains its own
single quotes (`'title'`, `'ENG'`, `'submission_deadline'`), which close and reopen the
shell string, silently degrading `'title'` to the bare identifier `title` — a *different
statement*, planned without complaint, `bash -n` clean. Now a quoted heredoc, verified
by round-trip. A gate that mangles its own probe text is indistinguishable from one that
works, right up until it certifies the wrong thing.

### The pre-fix falsifier is on record

run-driver-2, same DB and same run: **B2 GREEN** (`USING INDEX
tender_version_bid_parties_version`), **B1 RED** (`USING INTEGER PRIMARY KEY (rowid=?)`).
The index that *is* present passes; the walk no presence-check could see fails. That pair
is exactly what this gate was built to produce, and it is captured in the script header.
After the `lots_of` fix, B1 must flip to naming a real index — both
`sqlite_autoindex_lots_1` and a named `lots_*` index are accepted, so the gate stays
valid whichever route the fix takes.

### Deferred — a B7 once issue 115 lands

115 (16 big Tenders slow: `tender_detail` issues per-lot correlated subqueries, so cost
grows with lot count) is a second read-path defect of the same family — right rows,
wrong access pattern, invisible to every row-counting gate. It is *not* a plan defect
the current B-set can see: the per-lot subqueries may each be index-served and the read
still be slow, because the fault is how MANY of them run, not how each is planned.

So when proj-fix batches it, this gate grows a **B7** on the batched query's plan, and
that is a regression detector only — per the EQP asymmetry above, the *speedup* still
has to be proved with a clock. Blocked on the fix landing; nothing to assert against yet.

## Sequencing

110 is **done** — section I landed greenfield in `a2835b5` (it had never existed in the
committed suite; the prior session committed the issue and not the code). This is next.
Then re-run the fixed suite against a snapshot when nothing is serving, as the durable
record.

## THE CANONICAL POST-DEPLOY RUN — 2026-08-03 (run-driver, on the box)

`TDB_REV` unset and auto-derived as `1830d50c74a…`; 21 declared indexes;
`TDB_SNAPSHOT=/data/db/tender-db.db` (metadata only, `immutable=1`);
`TDB_PLAN_BIN=…/turso-bench/target-0.7.0/release/plan` (pinned turso `=0.7.0`);
`TDB_PLAN_DB=/data/scratch-lots/planschema.db`, a schema-only clone of prod's
catalogue (62 indexes = prod's 62, zero `sqlite_stat1` rows for the tables under
test). **28 pass, 1 fail, 0 no-input.**

```
B1  PASS  lots served by index sqlite_autoindex_lots_1
B1b PASS  same, on an after=49377 cursor page
B2-B6 PASS
C1  PASS  control still RED: SEARCH l USING INTEGER PRIMARY KEY (rowid=?)
```

**This is the result the issue was opened for**, and the shape of it matters more
than the greens: B1 green *with C1 red*, same probe, same DB, same run, the two
statements differing only in the cursor predicate. The instrument was shown to
discriminate at the moment it reported good news. A green without that is what
this issue spent its whole life arguing against.

The single FAIL was a bug in the gate, not in prod (below). Prod was not mutated.

### The stats precondition held for real

Prod's DB does carry a `sqlite_stat1`, but with exactly **one** row, for
`plan_notice` — a `plan_*` grouping-scratch table this gate already excludes.
**Zero** rows for `lots` / `tenders` / `organizations` /
`tender_version_bid_parties`. So the schema-only planning assumption is
representative for every table under test, and the precondition passed on
evidence rather than by default.

### Independent derivation agreement — the strongest provenance result here

run-driver reconstructed the deployed `lots_of` SQL by *reading* `read::lots`,
`pick()` and `version_predicates()` at `1830d50`. I obtained it by *dumping* the
builder through the `Query::rows` choke point. The two texts were diffed:
**byte-identical** (modulo the tender-id literal each of us chose). Two people,
two methods, one artifact — which is what makes the B1 statement trustworthy in a
way that no amount of careful retyping could.

## The two gate defects the run exposed — both fixed (`8058657`)

### 1. The SCAN detector never fired against this probe

It anchored `SCAN` to the start of the line. The probe prints EQP's raw columns:
`1 | 0 | 0 | SCAN lots`. The anchor never matched, so a real scan fell through to
the index-name branch. Measured against the committed gate: a bare `SCAN lots`
came back **NO-INPUT** — "not a recognisable scan" — never FAIL.

**The headline check of this file was inoperative.** B6 passed only because its
scan happens to be the correct plan: right verdict, no working check. Note the
shape of this — the gate was *built* around distrusting SEARCH-vs-SCAN as a
correlate, and then shipped with the SCAN half silently disabled.

The fix is **not** "make SCAN fail", and that distinction is the interesting part:

* `SEARCH l USING INTEGER PRIMARY KEY (rowid=?)` is the 13.2M-row **walk**, and it
  says SEARCH.
* `SCAN tenders USING COVERING INDEX tenders_current_published` is the **right**
  plan for `ORDER BY current_published_at DESC, id DESC LIMIT 50` — an ordered
  traversal that stops after 50 rows.

Failing the second would make the gate cry wolf about correct code, which is how
gates get switched off — the same reason `plan_*` indexes are excluded from
section A. So the discriminator is whether the access reaches rows **through an
index at all**: rowid/IPK on the target is red, a scan naming no index is red, and
a scan that does name one passes *while the report says it is an ordered full
traversal and not a seek*, rather than blurring the two into one word.

One parser now handles both the column form and the `|--SEARCH` tree form, and
section C uses it too — a control scored by different logic could agree with
section B for the wrong reason, and would not be a control.

### 2. Partial-index predicates were dropped from the expectation

The extractor's `\([^)]*\)` stopped at the first `)`, so `notices_unprojected`
compared as `notices(id)` while the on-disk DDL kept
`WHERE parse_state='parsed' AND projected=0`. Guaranteed mismatch.

The visible symptom was a bogus FAIL. **The dangerous half was silent:** with the
predicate stripped from the expectation, an index rebuilt with the *wrong*
predicate — `WHERE projected = 1`, serving nothing the read needs — compared equal
and **PASSED**. A silent no-check hiding behind a visible false alarm, which is
the worse of the two failure modes wearing the mask of the milder one. Verified
both ways against purpose-built DBs.

## A false claim in this gate's own header, corrected

The header implied the recorded pre-fix **RED** came from the B1 statement as
committed. run-driver planned that old text on turso 0.7.0 against two independent
DBs: it returns `SEARCH l USING INDEX sqlite_autoindex_lots_1` — **green** — with
or without the fix. The paraphrase had dropped the cursor predicate, and the
cursor predicate is the entire defect.

So **the gate's own recorded falsifier could not falsify.** The walk was real; the
attribution was not. Corrected in place rather than deleted — a gate that quietly
rewrites its history is worse than one carrying a wrong entry, and this is the
cleanest example in the file of why an artifact must be extracted rather than
restated. Section C now carries that weight properly, establishing the RED in-run,
from a statement of known provenance, every time it runs.

## What remains open

* **B2-B6 are still paraphrases** (114's point 1). B1/B1b are extracted; the other
  five carry exactly the drift risk that just proved fatal for B1's predecessor.
  This is now a demonstrated failure mode in this file, not a theoretical one.
* **The gate is structurally blind to issue 115** — see the B7 note above.
