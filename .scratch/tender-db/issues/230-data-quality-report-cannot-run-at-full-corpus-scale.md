# 230 — the data-quality report times out on every query at full-corpus scale, and reports empty instead of saying so loudly

Status: needs-triage — measured 2026-08-18 against prod (rev `cc0ef20`)
Kind: observability / tooling rot (a green that stopped being green without anyone noticing)
Blocked by: —
Relates to: 27 (the report this is, whose acceptance no longer holds), 120 (the app cannot bound an
expensive query — the same wall from the other side), 167 (capacity/abuse model), 226 + 228 (the same
lesson: a signal that cannot distinguish two states is not a signal)

## What

`crates/ingest/src/bin/data-quality` — the only *semantic* completeness measurement (per-era field
completeness, award linkage, results materialisation, TED↔DÖE merge) — cannot complete a single query
against prod. Run today with a valid token, on-box, against `localhost:8080`, queue idle, 01:55 UTC:

    query `versions`      failed (HTTP 408: query exceeded the 10s time limit)
    query `title`         failed (HTTP 408) … `buyer`, `value`, `cpv`, `deadline`, `winner`
    query `linkage`       failed (HTTP 408)
    query `density_can`   failed (HTTP 408) … `density_with`
    query `merge`         failed (HTTP 408)

**11 of 11.** Every section then printed with no rows, and section 4 printed the actively wrong-looking
"DÖE procedure Tenders: 0; merged with TED: 0" — a real zero and an unmeasured zero rendering
identically.

## Why it happened, and why it went unnoticed

Nothing regressed in the code. The queries were bounded *for the corpus that existed when issue 27
first ran them* (mid-backfill, documented in docs/research/data-quality.md). The corpus has since
reached 14.27M notices → 7.9M tenders → ~14.15M tender_versions, with 40.9M organization_mentions, and
a satellite-driven `GROUP BY` over the version layer is now a multi-minute scan. The 10s `/v1/sql` cap
— which exists precisely to protect the serving DB (issue 17/120) — refuses them all.

It went unnoticed because the tool is *descriptive by design*: no pass/fail, so it exits successfully
while reporting nothing. A scheduled run would have looked fine forever. That is the issue-226 shape
again — and it means the project has had NO semantic-quality measurement for however long the corpus
has been this size, while believing issue 27 delivered one.

## What NOT to do

Do not raise the cap or retry. `docs/agents/prod-box-reads.md` is explicit: a 408 bounds the wait, not
the work, and turso cannot interrupt a statement, so each retry stacks another uninterruptible scan.
Today's run alone spent ~11 × 10s of uninterruptible scan on the serving DB (deliberately, once, in a
low-traffic window, as issue 27's acceptance check) — it should not be repeated as-is.

## Fix direction

The dashboard already computes comparable aggregates at this scale, and it does so *in-process*: the
background refresher runs them on the isolated reader pool with no request deadline, gated by
`heavy_write_in_progress` and a change-gate so an idle server does not re-scan (issues 61/191). The
data-quality report wants the same treatment rather than a bigger hammer:

1. move the measurement server-side, onto the refresher's cadence, and expose the computed result
   (a panel section, or a `/v1/…` read of the stored numbers) — the binary then formats instead of
   scanning; or
2. keep it external but make each query bounded in the way the era queries already are — per-era, per
   satellite, with an id window, so each stays inside 10s; or
3. accept a sampled measurement (documented as such) — a stratified sample per era answers "what share
   carries a title" to within a known error at a fraction of the cost.

(1) matches how every other expensive number in this codebase is produced and is the recommendation.

**Whatever the fix, close the silent-emptiness half too:** the tool must exit non-zero, and say so in
one line, when a section is empty because its query failed rather than because the data is absent. A
measurement tool that cannot distinguish "zero" from "unmeasured" is the defect that hid this one.
