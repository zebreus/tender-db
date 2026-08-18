# 238 — /v1/sql answers "your query exceeded the 10s limit" when the truth is "no reader was free"

Status: message half FIXED and deployed 2026-08-18 (`7809efe`, corrected in `f1cc6d4`); the
AVAILABILITY half is open — two uninterruptible queries still deny the endpoint to all users
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

## CORRECTION (same day): the cause is runtime saturation, not readers

The diagnosis below — WAL gate + cold connection cost — is **wrong**, and worth leaving in place with
this correction on top rather than quietly rewriting, because the reasoning error is the instructive
part.

Measured on prod after the first fix deployed: `SELECT 1` returning 503 at 11.0 s while **two
`sql-exec` threads sat at 25 % and 16 % CPU**. `SQL_RUNTIME_THREADS = 2`, and both were pinned by full
scans abandoned minutes earlier. turso has no `interrupt()`, so `AbortOnDrop` cannot stop a
non-yielding aggregate: the computation runs to completion whether or not a client is still waiting.
New tasks were therefore **never polled at all** — readers were never even reached.

The clue I had and failed to weigh: the wedge persisted for MINUTES with an idle job queue. No
checkpoint explains that. I reasoned from the shape of `Readers::get` (permit, then WAL gate) to a
plausible story, and stopped when the story fit rather than when the evidence forced it.

The fix (rev `f1cc6d4`) reports three outcomes from how far the task got — never polled (saturation),
polled without a reader (readers/checkpoint), reader borrowed (genuine query timeout) — so the message
names the actual subsystem. "Busy" alone had already cost an hour on the wrong one.

## The availability gap this exposes — still open

Issue 17's isolation goal HELD, exactly as its own comment predicted ("can pin at most this many
threads and never starves the rest of the server"): throughout the wedge, `/health` served in 0.4 ms
and `/v1/tenders` in 2.6 ms. That design decision is vindicated.

What is NOT bounded is `/v1/sql`'s own availability. **Two uninterruptible queries deny the endpoint to
every user**, and the per-user cap of 2 concurrent cannot help, because the pinning outlives the
request that caused it — the caller disconnects, the thread keeps computing. One analyst's pair of
full scans is a total outage of the analyst surface for as long as they run.

Options, none free:
- **More threads** — raises the ceiling, does not remove it, and spends CPU the rest of the server may want.
- **Admission control on cost** — refuse a query whose plan looks like a full scan of a huge table.
  Needs a plan check (`EXPLAIN QUERY PLAN` is available and cheap) and a policy; would have refused
  every query that caused this.
- **A per-user thread budget** so one token cannot occupy every worker.
- **Accept and document** — the endpoint is best-effort, and the 503 now says so honestly.

Recommendation: the plan check, because it attacks the cause (unbounded scans admitted at all) rather
than the symptom, and because the same check would have spared issue 230's eleven 408s.

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
