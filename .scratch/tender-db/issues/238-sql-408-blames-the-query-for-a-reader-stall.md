# 238 — /v1/sql answers "your query exceeded the 10s limit" when the truth is "no reader was free"

Status: needs-triage, RAISED — found 2026-08-18, code-confirmed; now BLOCKING issue 100's award
verification, so it costs a data investigation and not just operator patience
Kind: misleading diagnostic (the error names the wrong cause) + cold-connection cost
Blocked by: —
Relates to: 17 (the isolated SQL runtime), 51 (abandon on disconnect / slot bounding), 230 (whose
opening symptom was "11 of 11 queries return 408" — partly this), 07 (the endpoint)

## What happened

While a `project` job was finishing, every `/v1/sql` request returned

    {"error":{"message":"query exceeded the 10s time limit","status":408}}

including **`SELECT 1`**, at a consistent 11.0 s. `SELECT 1` touches no table and cannot cost 10 s, so
the message was false. Minutes later, with the queue idle, the first `SELECT 1` took **7.36 s** and the
second **0.8 ms**.

Public reads were unaffected throughout — `/health` 0.4 ms, `/v1/tenders?limit=2` 2.6 ms,
`/health/deep` green. The problem is confined to the sandbox path.

## The cause, from the code (`crates/app/src/v1/sql.rs`)

    let handle = sql_state.runtime.spawn(async move {
        let reader = readers.get().await…;                       // acquisition — NOT in the inner timeout
        match tokio::time::timeout(timeout, execute(&reader, &sql)).await { … }
    });
    let outcome = tokio::time::timeout(timeout + TIMEOUT_GRACE, handle).await;   // covers BOTH

The **inner** timeout correctly wraps only `execute`. The **outer backstop** wraps the whole spawned
task, so it also covers `readers.get()`. When acquisition alone exceeds the backstop, the handler
reports the one thing it knows how to report: a query timeout. It cannot distinguish "waited 11 s for a
reader" from "ran 11 s of SQL", and both render as the latter.

Cold acquisition on this 441 GB file measures ~7 s, i.e. most of the 11 s budget — so the sandbox has
almost no headroom, and any write contention pushes it over.

## Why this is worth fixing beyond tidiness

The message actively misleads the operator into fixing the wrong thing. I hit this an hour ago, and did
exactly what the message told me to do: assumed the query was too expensive, narrowed it, split it into
two, then broke it into an id list — three rewrites of a query that was never the problem, and one
wrong conclusion recorded ("0 notices with profile eforms-de-1.0") because an error body parsed as an
empty row set.

It also partly reinterprets issue 230's opening symptom. That issue starts from "11 of 11 queries return
HTTP 408", which was read as eleven too-slow queries. The windowed measurements later proved those
queries genuinely do take 17–90 s each, so the conclusion held — but the FIRST 408 of any run would have
been inflated by acquisition, and nothing in the report could have told the difference.

## It is already blocking other work

Issue 100's next step is counting results whose ORIGIN notice is DE-1.x. That count could not be
obtained: the endpoint failed the same query it had answered a minute earlier, intermittently, with the
408 this issue is about. Two distinct obstacles were in play there and it is worth keeping them apart —
one is this issue (an acquisition stall wearing a query-timeout message), the other is an
index-choice inversion where adding a `kind` predicate moves the planner off a PK seek onto
`notice_sections_kind`. Only the first is in scope here, but while it persists no on-box investigation
can tell "my query is wrong" from "the backend was busy", which is precisely the confusion that makes
this worth fixing before the next investigation rather than after.

## Fix directions

1. **Time acquisition separately and name it.** Wrap `readers.get()` in its own timeout and return a
   distinct status — 503 with `Retry-After`, message "sql backend busy, no reader available" — so a
   stall reads as a stall. Then the backstop measures only execution, which is what it claims to measure.
2. **Pay the cold cost once.** A warm/pinned reader for the sandbox pool (or a pool minimum) removes the
   ~7 s first-query penalty. Measure before and after; a 7 s constant against a 10 s budget is the whole
   fragility.
3. **Consider whether the budget should exclude queueing at all** — a user's 10 s should probably be 10 s
   of their own query, not 10 s shared with whatever the write side is doing.

## Acceptance

- `SELECT 1` never returns 408. If no reader is free it returns 503 with a retry hint.
- A recorded measurement of cold vs warm acquisition, before and after whatever (2) becomes.
- The `/v1/sql` docs say which of the two limits a caller is hitting, so the next operator does not
  rewrite a working query.
