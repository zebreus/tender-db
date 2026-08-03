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

Cheap partial check available before the full fix: plan each of B2-B6 against a DB
where the index it names is ABSENT, and confirm each goes RED. Any that stays green is
unfalsifiable and is asserting nothing. That is a fraction of the work of part 2 and
would tell us how much of the gate is real today.

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

## Note

Filed because 112's plan half is the part that catches the defect class nothing else sees,
and it currently rests on a file someone else owns and may reasonably delete. The gate will
say so when that happens. This issue exists so that saying so leads to a fix rather than to
a permanently-yellow line in a report.
