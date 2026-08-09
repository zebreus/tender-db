# 47 — Reconcile observed rate-limiting vs the documented ~10 rps (LOW)

Status: resolved-explained (2026-08-09, orchestrator) — no config change needed; every
part of the gap is attributed from git history, and the two contributing defects were
each fixed days later by their own issues:

1. **The response shape** (plaintext "Wait for 2s" + `retry-after` + `x-ratelimit-after`):
   tower_governor's DEFAULT error handler. The probe-time build (`eb77622`, 2026-07-21
   11:48) mounted `GovernorLayer::new(limits)` with no `error_handler`; the JSON envelope
   (`governor_json_error`) landed 2026-07-23 in `7c78d62` — issue 51, two days AFTER the
   probe. Today's 429 is the JSON envelope, unit-tested at v1/mod.rs (the
   `TooManyRequests` shape test).
2. **The rate was never misconfigured**: `per_second(10).burst_size(50)` at the probe-time
   rev, identical today. CONTEXT.md's "≈10 rps generous" and /docs' "~10 req/s per client
   (burst 50)" both agree with the code.
3. **The early refusal (~2–6 requests, multi-second wait)**: bucket SHARING, not a strict
   limit. At probe time the governor keyed on the FIRST `X-Forwarded-For` entry
   (client-spoofable; behind nginx's `$proxy_add_x_forwarded_for` the first entry is
   whatever the client sent) with a `"direct"` fallback shared by ALL unproxied traffic.
   Agent probes run on the box against 127.0.0.1:8080 — every such probe (and any other
   on-box client active that day, which 2026-07-21 was full of) shared ONE bucket, so a
   prober inherited a pre-drained bucket and saw refusals after a handful of requests
   with an accumulated multi-second wait. The spoofable half was fixed the same evening
   (`62f7255`, issue 44: trusted `x-real-ip` preferred). The shared `"direct"` bucket for
   unproxied traffic remains by design: it is reachable only from the box itself — public
   traffic always arrives through nginx with trusted headers, per-client buckets, burst 50.

Acceptance met: observed vs documented gap explained; config and docs already agree, so
neither needed correction. Operators probing from the box should expect to share the
`"direct"` bucket (or set `x-real-ip` per probe if isolation matters).
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
