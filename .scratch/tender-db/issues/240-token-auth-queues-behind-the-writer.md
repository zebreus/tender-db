# 240 — authenticating a bearer token took the WRITER connection, so every gated request hung for the whole of a fold

Status: DONE 2026-08-19 — deployed (`be8cba6`) and re-probed on prod DURING a live re-fold: a real
query returned 200 in 1.6 ms and an invalid token 401 in 0.6 ms, against no response in 120 s / 25 s
before. Follow-ups split out to issue 241.
Kind: availability defect on the auth path (not a query-cost problem)
Blocked by: —
Relates to: 61 (/health reads the in-memory cursor, which is why it stayed green), 238 (the sandbox's
408/503 attribution — the backstop that could not help here), 17 (SQL runtime isolation), 07 (the SQL
endpoint), 45 (the sandbox's allow-list)

## What happened

While a `refold-sections` job held the writer, `/v1/sql` stopped answering — not slowly, at all:

    SELECT COUNT(*) FROM tender_version_lot_group_members   (233 rows)  →  no response in 120 s
    SELECT 1                                                           →  no response in  30 s
    ...with a DELIBERATELY INVALID token                               →  no response in  25 s
    /v1/sql/schema            (no auth)                                →  200 in 1 ms
    /v1/tenders?limit=1       (reader pool)                            →  200 in 5 ms
    /health, /health/deep, /metrics                                    →  200 in ≤2 ms

The invalid-token row is the one that localises it: a request that must end in 401 also hung, so nothing
about SQL cost, the allow-list, the rate limiter or the semaphore is involved. The stall is in the
`AuthUser` extractor, before the handler.

## Why

`Db::authenticate_token` opened with `self.conn().await` — the single writer connection behind a
`tokio::Mutex` — and used it for BOTH halves: the hash lookup and the best-effort `last_used_at` touch.
A projection holds that mutex for the whole of a fold (25 minutes for the `GroupComposition` cohort,
hours for a corpus rebuild), so every token-bearing request queued behind the job.

**And nothing timed it out.** `/v1/sql`'s backstop (issue 238) is inside the handler; an extractor that
never returns never reaches a handler. So the failure mode was an open connection with no response and
no error — the worst shape for a client, and invisible to every probe we have, because `/health` reads
the in-memory cursor by design (issue 61) and the ordinary read path uses the reader pool.

Blast radius: every endpoint taking `AuthUser` — and also every endpoint taking `Option<AuthUser>`,
which calls the same function whenever a token header is present. A caller who sends a token therefore
hung on endpoints that do not even require one, while an anonymous caller sailed through.

## The fix

1. **The lookup runs on a reader.** It is a pure read; readers run in parallel with the writer over WAL.
2. **The touch takes the writer only if it is free** (`try_lock`). The doc comment already said the touch
   was best-effort and that "losing one write is preferable to failing an otherwise valid request" —
   `try_lock` is what makes that sentence true. Waiting for the writer was the outage; the cost of
   skipping is a dashboard timestamp missing for uses that happen during a job, and nothing else.

Regression test: hold the writer exactly as a fold does, then authenticate, wrapped in a 5 s timeout so
a regression FAILS rather than hangs. It also pins the two-part behaviour: touch skipped while the
writer is held, recorded once it is free.

## Audit of the sibling paths (done)

Every other `self.conn()` in `accounts.rs` is a genuine write (create/delete user, create/delete
session, create/revoke token). Session resolution and `user_credentials` already read via the pool. So
`authenticate_token` was the only read on the writer, and the class is now clear in this module.

## What this says about our probes, which is the more important half

A 25-minute total outage of every authenticated endpoint left **no trace** in `/health`, `/health/deep`,
`/metrics` or the job record, and was found only because a human-driven query happened to run during a
fold. Two follow-ups worth their own issues:

- **`/health/deep` should exercise the writer-dependent path**, or something should, with a bound. A
  liveness probe that is instant by construction cannot see a class of outage that is entirely about
  waiting.
- **No timeout exists around extractors.** A bounded auth (e.g. 2 s, then 503) would have turned a hang
  into a legible error even without knowing the cause. That is defence in depth, not a substitute for
  the fix above.

## Acceptance

- Authentication answers while a fold holds the writer (test above, and re-probed on prod during the
  next long job after deploy).
- `last_used_at` still records uses outside jobs (existing test, unchanged).
