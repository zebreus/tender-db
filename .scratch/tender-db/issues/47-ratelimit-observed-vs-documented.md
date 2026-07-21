# 47 — Reconcile observed rate-limiting vs the documented ~10 rps (LOW)

Status: ready-for-agent
Severity: LOW (investigate; likely mis-attribution, not a misconfig)

A live probe (2026-07-21) reported the anonymous surface refusing after
~2–6 requests with a re-arming "Wait for 2s" penalty (retry-after: 2,
x-ratelimit-after: 2, plaintext body) — materially stricter than
CONTEXT.md's "≈10 rps generous".

Owner note: the governor IS configured at the documented rate
(RATE_PER_SECOND = 10, crates/app/src/v1/mod.rs:58) and returns JSON, not
that plaintext body. The observed "Wait for 2s" plaintext + custom header
looks like a DIFFERENT limiter — most likely the /v1/sql per-token quota
(MAX_CONCURRENT = 2, PER_HOUR = 300) or a compounding penalty under
sustained probing — not the general governor.

Task: attribute the observed behavior precisely (which layer emits the
"Wait for 2s" response; whether general /v1 GETs really cap at ~2–6/s or
only the SQL endpoint does), with a gentle, bounded probe. If the general
surface is genuinely stricter than 10 rps, fix the config; if it's the
SQL limiter behaving as designed, just align the CONTEXT.md wording /
docs so the two limiters aren't conflated. Do NOT hammer prod — a few
spaced requests, or reproduce against a local instance.

Acceptance: the observed vs documented gap is explained; either the
config or the docs corrected so they agree.
