# 114 — the plan gate must DERIVE its SQL and its engine from the artifacts, never restate them

Status: open — filed 2026-08-03 (sdk-vendor), scope widened same day.
**Part 2 (source the SQL from the builder) raised from LOW to HIGH on 2026-08-03**: it
stopped being a hardening and became the fix for a DEMONSTRATED hole in 5 of the 6
hot-read checks. See "Part 2 is no longer hypothetical" below. Part 1 (the engine)
remains LOW — it fails yellow, not green.
Kind: verification durability
Blocked by: — (112 is functional today; this is its durable replacement)
Relates to: 112 (the gate), 111, 107, 110/102 (the same artifact-vs-proxy error)

## The one principle, two instances

112 asserts things about production using two inputs it does not own: the **engine** that
produces a plan, and the **SQL** whose plan is produced. Today it *restates* both — it
borrows a scratch probe for the engine, and it hand-writes SQL to match what `read.rs`
emits. Each restatement can drift from the artifact it stands for, silently, while the
gate keeps reporting green.

This issue covers both, because they are the same defect and the same fix shape: **derive
from the artifact, never paraphrase it.**

| input | restated as | drifts when | consequence |
|---|---|---|---|
| the planning **engine** | a scratch probe binary on one box | the probe is deleted, or its pinned turso diverges from the server's | section B → no-input; the plan half quietly stops covering anything |
| the **SQL** under test | statements hand-written to match `read.rs` | `read.rs`'s query changes and the gate's string does not | section B stays GREEN while the read that actually runs regresses |

The second is the more dangerous of the two — it fails *green* rather than *yellow*.

## The dependency

112's plan half (`canonical-verify/hot_read_plans.sh` section B) gets its query plans
from run-driver-2's probe at `/opt/tender-db/turso-bench/plan` — a **scratch artifact on
one box**, built for a one-off experiment with a pinned `turso ="=0.7.0"`.

It works today and it was the right call to wire it: right engine, deployed version,
independent of the app, and no new build. But it is not a durable input.

## What happens when it goes

Someone cleans up `/opt/tender-db/turso-bench`, or reprovisions the box, or the pinned
turso drifts from the version the server actually links. Then section B reports
**no-input** — loudly, and with the reason, which is the gate behaving correctly.

That is exactly the failure mode 112 was designed to have instead of a false green, so
nothing breaks silently. **But the plan half stops covering anything**, and a no-input
that recurs every run is the kind of thing a team learns to scroll past. The gate degrades
to its presence/DDL half — which, per 112's whole argument, is the *correlate* that was
already green while the site served 2.2s pages.

So the risk is not a wrong answer. It is the gate quietly reverting to the weaker check it
was built to replace, while still printing a mostly-green report.

## Fix — the app-side EQP diagnostic

A small read-only diagnostic that runs `EXPLAIN QUERY PLAN` through the server's own
connection pool and returns the plan text. ~30 lines. Metadata-only (EQP compiles without
executing), so it is live-safe and needs no snapshot and no quiet window.

For *plans specifically* this has the strongest claim to being load-bearing of any source
available: it is literally the engine, at the version, with the settings, that serves
production traffic. The probe is a faithful stand-in; the app is the thing itself.

Wiring is already done on 112's side — it consumes any `TDB_PLAN_CMD` that reads SQL on
stdin and writes a plan on stdout, so this drops in with no change to the gate.

### The one thing to get right

An app-side source is *less* independent than the probe: the gate would be asking the
application about the application. For plan verdicts that trade is acceptable — a plan is
a statement about the engine's behaviour, and the serving engine is the authority on it,
so there is no "grading its own homework" in the way there would be for a *correctness*
claim. But it means:

- the diagnostic must run the **real read's SQL**, not a hand-written approximation of it
  that could drift from what `read.rs` actually issues; and
- it must not be built on top of anything 112 also consumes as an expectation, or the two
  halves stop being independent.

Keep the probe path working as the cross-check. Two sources that agree are worth more than
either alone, and they fail differently — the probe drifts by *version*, the app by
*deployment*.

### Stats precondition still applies

112 refuses a plan verdict unless the planning DB's `sqlite_stat1` state is established and
empty for the tables under test. An app-side source plans against the **live** DB, so the
diagnostic must expose which DB it planned against (or the gate must be able to establish
its stats state some other way). If prod is ever `ANALYZE`d, this precondition is what stops
the plans silently ceasing to describe production — do not drop it in the port.

## Part 2 is NO LONGER HYPOTHETICAL — it has now failed in production use

Everything below was written as a risk. On 2026-08-03 it happened, was measured, and
was caught only by accident.

112's B1 was a hand-written approximation of the `lots_of` read. run-driver planned
that exact text on turso 0.7.0 against two independent DBs:

```
B1 as committed  ->  SEARCH l USING INDEX sqlite_autoindex_lots_1   GREEN
                     ... with OR without the fix applied
```

The paraphrase had dropped the **cursor predicate**, and the cursor predicate is the
entire defect. So:

* **B1 could never have failed.** The gate's headline check was unfalsifiable by
  construction, for its whole life.
* **The RED recorded in the gate's own header could not have come from it.** The walk
  was real; the attribution was fiction. A gate carrying a falsifier artifact it did
  not produce.
* It went green over the live 2.2s defect — the **fourth** false green in 112's story,
  and the only one authored inside the gate itself.

It was not caught by the gate, by review, or by the pre-run discipline. It was caught
because a fix landed, the SQL had to be re-synced, and someone thought to plan the old
text as a control.

**B2-B6 are the same construction, unexamined.** Five of six hot-read checks are still
paraphrases whose ability to fail has never been demonstrated. On the evidence of B1
the prior should be that at least one of them is also unfalsifiable — they were written
the same way, by the same method, at the same sitting.

The interim mitigation is that they are now *labelled* as paraphrases in the script.
That converts a hidden hole into a known one; it does not close it.

### The static audit — done 2026-08-03, no box required, and it found worse

Rather than leave B2-B6 as a prior, their paraphrases were compared against the SQL the
deployed code at `1830d50` actually issues. This needed no turso and no box — just
reading the builders. Result: **every one of the five differs, and one of them is
asserting a read that no longer exists.**

**B5 — asserts a query the deployed code NEVER ISSUES. Vacuous.**
112's table justifies B5 as "the Phase-1 mention resolver … full scan of ~30M per new
mention". That read was **deleted by issue 19**. `canonical.rs` is explicit about it:
the old per-mention lookup `WHERE country IS ? AND identifier_kind = ? AND identifier
= ?` was O(n²) and is gone, replaced by an in-memory `org_of` map. What the deployed
resolver actually issues, once, at construction:

```sql
SELECT id, country, identifier_kind, identifier FROM organizations
 WHERE identifier IS NOT NULL          -- an intentional ONE-TIME FULL SCAN
```

So B5 plans a synthetic statement, goes green, and that green protects nothing. Note
also the paraphrase says `country = ?` where the historical query said `country IS ?` —
different planner behaviour around NULLs, so it was not even a faithful copy of the
read it was standing in for.

**And the naive fix would make it worse:** point B5 at the real resolver query and it
goes RED, because a full scan is the *correct* design there — the table is preloaded
once, not probed per mention. That is the same cry-wolf trap as the SCAN detector.
B5 should be **deleted, not repaired**, and 112's target table amended: the resolver is
no longer a hot indexed read.

**B6 — drops a predicate that bears on index usability.**
```
real (lib.rs):  … FROM tenders t WHERE t.current_published_at IS NOT NULL
                  ORDER BY t.current_published_at DESC, t.id DESC LIMIT ?
                  + a correlated title subquery per row
B6 plans:       SELECT id FROM tenders ORDER BY current_published_at DESC, id DESC LIMIT 50
```
Missing the `IS NOT NULL` filter and the per-row subquery. B6 also names the wrong home
for the read — it is in `lib.rs`, not `read.rs`; `read.rs`'s tenders list orders by
`t.id`, not by `current_published_at` at all.

**B3 / B4 — column list differs, and it is not cosmetic.**
Real: `SELECT id, source, projection_epoch FROM tenders WHERE procedure_key = ?`.
B3 plans `SELECT id …`. With only `id`, `tenders_procedure_key` is a **covering** index;
with the extra columns it is not. Same index, different plan text and different work per
row — so the check is not planning the read it claims to.

**B2 — drops the JOIN.** Real read joins `organizations` for the party name; B2 plans
the bare table. The target-table access is probably unchanged, so B2 is the mildest of
the five, but it is still not the artifact.

**Severity order for the fix: delete B5, correct B6, then B3/B4, then B2.**

This audit cost minutes and found a vacuous check that had been reported as a PASS in a
canonical run. It is the strongest available argument that part 2 is not hardening: five
of six hot-read checks were paraphrases, and inspecting them turned up one asserting
nothing and one pointed at the wrong file.

Still worth doing on the box afterwards, as the independent confirmation: plan each
surviving B-check against a DB where the index it names is ABSENT and confirm each goes
RED. Reading the code proves the statement is wrong; only running it proves the check
can fail.

## Fix, part 2 — source the SQL from the builder

Section B's statements are currently hand-written approximations of what `read.rs`
emits. 112 carries this as a **blocking pre-run item** — before each post-fix run,
B1 must be replaced with the exact SQL the builder emits, dumped from `q.sql` rather
than retyped. That is a manual discipline, and manual disciplines lapse.

The durable fix is to stop restating: have the gate obtain the statements from the
builder itself (a dump mode on the query builder, or a test fixture the builder writes
and the gate reads), so a change to `read.rs` either updates what the gate plans or
makes the gate say it can no longer establish its input. Either is acceptable; silently
planning the previous query is not.

The extraction method is already proven and cheap — it was used to sync B1 on
2026-08-03 and took minutes: env-gate a dump of `self.sql`/`self.params` inside
`read.rs`'s `Query::rows`, the single choke point every read passes through, then call
the read. Two independent derivations of the resulting SQL (a dump by sdk-vendor, a
by-hand reconstruction from the builder source by run-driver) came out byte-identical,
so the method is sound. What is missing is not a technique but a **standing seam**: the
dump was a temporary patch in a throwaway worktree, so nothing stops the next drift.

Note this composes with part 1: an app-side diagnostic that plans **the real read**
(rather than a string handed to it) solves both halves at once — the engine is the
serving engine and the SQL is the emitted SQL. That is the strongest end state and
probably the reason to do them together.

## Acceptance

- With the app-side source configured, 112 section B produces the same verdicts as the
  probe on the same DB — including the pre-fix `lots_of` RED if run against a pre-fix
  snapshot.
- Removing `/opt/tender-db/turso-bench/plan` entirely leaves section B fully functional.
- The diagnostic is read-only and executes no data query (assert it cannot be made to run
  the SELECT itself, only plan it).
- The stats precondition still fires: point it at an `ANALYZE`d DB and section B reports
  no-input, not a verdict.
- **Change the `lots` query in `read.rs` and re-run the gate without touching it:** it
  must either plan the NEW query or report no-input. Continuing to report green against
  the old string is the failure this part exists to prevent.
- **Every B-check can be shown to fail.** For each hot read, planning it against a DB
  without the index it names must produce RED. B1's predecessor passed review while
  being incapable of failing; "it goes green on a healthy DB" is not evidence that a
  check works, and this acceptance line is what distinguishes the two.

## Part 2, second half — the SET of checked reads is derivable too, and deriving it is the whole point

Everything above fixes *how* each check states its query. It does not fix **which queries
get a check at all**, and that is where the day's most expensive miss actually came from.

112's six checks were enumerated **by recall**. The consequence, measured on 2026-08-03:

* B5 asserted a read issue 19 had **deleted** — a check on a query nothing issues.
* Nothing at all asserted `read::organizations`, which walked 25.3M rows on an
  unauthenticated public endpoint (22.0s with `?country=`, 99.08s with `?kind=`).

Same table, same file, same sitting. One check pointed at a corpse while the live defect
of exactly this gate's target class sat uncovered. Fixing each check's SQL would not have
found it: **a paraphrase can be corrected, a missing check cannot.**

### The derivation anchor: `read_items`

There is already a single exhaustive statement of what the paginated read set IS —
`app/src/v1/mod.rs`:

```rust
pub async fn read_items(collection: Collection, conn, filter: &Filter, scope: Scope) -> …
    match collection {
        Collection::Tenders       => read::tenders(conn, filter, scope),
        Collection::Lots          => read::lots(conn, filter, scope),
        Collection::Organizations => read::organizations(conn, filter, scope),
        Collection::Notices       => read::notices(conn, filter, scope),
    }
```

Its own doc comment says it: *"the single evaluation point for the filter predicates: the
list endpoint, the SSE snapshot and the SSE diff probe all come through here."* Four
variants, four builders, and **the compiler enforces the exhaustiveness of that match**.

So the completeness guarantee should be a compile error, not a convention and not a grep:
key the gate's statement registry by `Collection` and build it with a `match` on it. Add a
fifth collection and the registry **fails to build** until it is covered. That is strictly
stronger than any source-text count (`grep "Scope::Page { after, limit }"` and friends),
which a rename or a rustfmt change can silently defeat.

Reads *outside* that match are not covered by this derivation and must be named as such
rather than assumed absent — `changes_since` (its own `cursor`/`limit`, not a `Scope`),
and the `tender_detail` fan-out (`results_of`, `lots_of`, the bid-parties read). They need
their own rule; what they must not have is silence.

### A read is not a statement — the organizations lesson, generalised

Coverage per *read* would still have missed half of what was measured. One builder emits a
different statement per filter, and the plans differ in kind:

| statement | plan | measured |
|---|---|---|
| `organizations` + `country` | rowid walk; a row-value cursor gives it `organizations_identity` | 22.0s, **fixable** |
| `organizations` + `kind` | no index can serve it — `organizations_identity` is `(country, identifier_kind, identifier)` and `kind` has no leading `country` | 99.08s, **unservable as schema'd** |

Same function, same table, one check each would have been wrong. The unit of coverage is
therefore **(collection × filter shape × cursor position)**, not the read.

Every one of those is publicly reachable: `Params` accepts `source, country, cpv, buyer,
winner, status, min_value, max_value, kind, tender` on any collection endpoint, plus
`cursor`. Full cross-product is 2^10 per collection and pointless; the bounded enumeration
that actually catches this defect class is

* the **empty** filter, plus **each single filter alone** (11 per collection), and
* each of those at **`after = 0` and `after > 0`** — a first-page-only assertion certifies
  a half-fix as whole (measured: `/v1/lots?tender=&after=N` stayed at 2237ms under an
  `after == 0`-only fix),

= 88 candidate statements, **deduplicated by emitted text** (a filter the builder ignores
produces byte-identical SQL, so the distinct set is far smaller). 88 EQP compiles is
nothing; the cost of this enumeration is entirely in the triage below, not in running it.

Documented blind spot, to be logged rather than silently dropped: filter **combinations**.
They matter only where a composite index could be unlocked by a second predicate. Out of
scope for now, and said out loud, per "no silent caps".

### Triage — enumeration must not resurrect the cry-wolf problem

Mechanical enumeration collides head-on with the gate's rule 4 (*a check needs an
achievable pass state*). `?kind=` is precisely the case: red, with **no reachable green**,
because no index on the table can serve it. Enumerate everything and assert everything and
the gate acquires a permanent alarm — which is how gates get switched off, and then catch
nothing at all.

So each **distinct** derived statement gets exactly one of three dispositions, and the
disposition is recorded next to it:

| disposition | meaning | report state |
|---|---|---|
| **asserted** | an index-served plan is reachable | pass / fail, as today |
| **known-unservable** | no access path exists under the current schema; tracked as a defect, not as a gate failure | reported, never counted as fail |
| **excluded, with reason** | e.g. correct-by-design full scan (the `org_of` preload) | reported once, with the reason |
| *(unclassified)* | a statement the enumeration produced that nobody has dispositioned | **no-input — LOUD**, never silence |

The last row is the load-bearing one. A newly-added filter or collection must arrive as a
*noisy unknown*, because the failure mode this whole part exists to fix is a read entering
the system and nothing noticing.

### Implementation shape

1. `#[cfg(test)] pub(crate) fn <read>_statement(filter, scope) -> (String, Vec<Value>)` per
   builder — `read::lots_statement` (proj-fix, `2ea1b23`) is the prototype and the pattern:
   the body moves into a `*_query` that returns the assembled `Query` unrun, `lots()` calls
   it, so test and production share **one** builder rather than a copy.
2. A registry built by `match` on `Collection`, so a new collection is a **build failure**.
3. An enumerator that walks (collection × single filter × cursor position), inlines each
   `?` with the value the builder itself bound, dedupes by text, and writes the set as a
   fixture.
4. A test that fails when the checked-in fixture is **stale** — that is what stops the gate
   planning yesterday's SQL, and it needs no box, no turso and no snapshot.
5. 112 consumes the fixture instead of its hand-written `READS` table.

### PROTOTYPED AND RUN, 2026-08-03 (sdk-vendor) — 88 candidates, 56 distinct, 24 filter-specific

Built end-to-end in a detached worktree at `2ea1b23` and run. The seam patch is checked
in beside the gate as `canonical-verify/read-statement-seams.patch` — it splits
`tenders`/`organizations`/`notices` the way `2ea1b23` split `lots` (body into a
`*_query` returning the unrun `Query`, plus a `#[cfg(test)] *_statement`). Pure
function-boundary extraction: **all 56 `store` lib tests pass unchanged**, including
proj-fix's own plan tests.

Numbers came out exactly as the design predicted: 11 filters x 2 cursor positions x 4
collections = **88 candidates**, deduplicating by emitted text to **56 distinct
statements**, of which **24 are filter-specific** (the rest are a filter the builder
ignores, emitting the unfiltered statement byte-for-byte).

**A trap the prototype hit, which any fixture format inherits.** The builders carry
`--` comments INSIDE their SQL. Flatten a statement to one line for a line-delimited
fixture and the first comment silently comments out everything after it; turso then
rejects the whole statement as `"incomplete input"`. Quote-aware comment stripping is
therefore mandatory in the extractor, not a nicety — and anyone hand-syncing B2-B6 by
copy-paste is one collapsed newline away from planning a truncated statement.

#### What the plans say (local lab, turso 0.7.0, `Db::open` catalogue + all deferred indexes)

Index-served drivers — 3 of the 24:

| endpoint | driver |
|---|---|
| `/v1/lots?tender=` | `SEARCH vl USING INDEX sqlite_autoindex_tender_version_lots_1` — issue 115's fix |
| `/v1/notices?kind=` | `SEARCH notices USING INDEX notices_profile (profile=?)` |
| `/v1/lots?source=` | `SEARCH t USING INDEX tenders_island (source=?)` |

The other 21 filter-specific statements drive by a rowid access on the outer table.
**That is a candidate list, not 21 defects**, and the reason is the same ambiguity this
gate was built around: `SEARCH o USING INTEGER PRIMARY KEY (rowid=?)` is printed for a
genuine one-row lookup AND for a full walk. `/v1/organizations?buyer=` is the clean
example — the filter is `o.id = ?`, so that rowid access is a point lookup and there is
nothing wrong with it. Plan text alone cannot separate the two; the statement's
semantics decide. Cross-check that the lab is honest: `/v1/organizations?country=`
plans as a rowid access here even with `organizations_identity` present — which is
exactly what run-driver measured on prod at 22.0s. Adding the three projection-built
indexes (`organizations_identity`, `changes_entity_cursor`, `notices_unprojected`)
changed **zero** verdicts, so none of these are artefacts of a thin local catalogue.

**This is precisely why the triage step is load-bearing rather than bureaucracy.** A
mechanical sweep can enumerate the set and produce the plans; only a human (or a clock)
can say which rowid access is a lookup, which is a walk with a reachable green, and
which is unservable as the query is shaped today.

#### A second finding, of a different kind: silently ignored filters — now issue 118

**16** (collection, parameter) pairs are a query parameter the API **accepts and then
ignores**, emitting the unfiltered statement byte-for-byte: 7 on `/v1/organizations`,
8 on `/v1/notices`, and `/v1/tenders?tender=`. (An earlier note here said 15; the exact
count from the statement comparison is 16.) `Params::filter` builds one `Filter` for
every collection and each builder uses only the fields meaningful to it, so anything a
builder does not read is dropped in silence.

That is a correctness question, not a plan question: a client asking for
`organizations?cpv=4521` gets every organization and no indication that its filter was
dropped. **Filed as issue 118, as an observation for triage rather than an asserted
defect** — it may be deliberate.

What belongs in THIS issue is how it was found: not by reading the code with a question
in mind, but as a by-product of DEDUPLICATING the mechanically enumerated set. The 88
candidates collapsed to 56 distinct statements, and the collapses were precisely the
parameters that change nothing. Nobody had this on a list to check. That is the whole
argument of part 2, arriving as evidence rather than as an assertion.

### Sequencing note (2026-08-03, sdk-vendor)

Step 1 touches `tenders`/`organizations`/`notices` — the exact functions proj-fix is
changing for the DoS-class fixes. The seam is a pure function-boundary extraction, so the
cheapest and least conflict-prone moment to add it is **inside those fixes**, not in a
parallel branch afterwards. `lots` already has its seam and needs nothing.

## Note

Filed because 112's plan half is the part that catches the defect class nothing else sees,
and it currently rests on a file someone else owns and may reasonably delete. The gate will
say so when that happens. This issue exists so that saying so leads to a fix rather than to
a permanently-yellow line in a report.
