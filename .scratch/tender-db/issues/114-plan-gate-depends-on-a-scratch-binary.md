# 114 — 112's plan half depends on a scratch binary; make the app the standing plan source

Status: open — filed 2026-08-03 (sdk-vendor). LOW priority, but tracked so the lapse is a
decision rather than a drift.
Kind: verification durability
Blocked by: — (112 is functional today; this is its durable replacement)
Relates to: 112 (the gate), 111, 107

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

## Acceptance

- With the app-side source configured, 112 section B produces the same verdicts as the
  probe on the same DB — including the pre-fix `lots_of` RED if run against a pre-fix
  snapshot.
- Removing `/opt/tender-db/turso-bench/plan` entirely leaves section B fully functional.
- The diagnostic is read-only and executes no data query (assert it cannot be made to run
  the SELECT itself, only plan it).
- The stats precondition still fires: point it at an `ANALYZE`d DB and section B reports
  no-input, not a verdict.

## Note

Filed because 112's plan half is the part that catches the defect class nothing else sees,
and it currently rests on a file someone else owns and may reasonably delete. The gate will
say so when that happens. This issue exists so that saying so leads to a fix rather than to
a permanently-yellow line in a report.
