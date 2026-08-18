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

**CORRECTION (2026-08-18, same day, owner).** The first version of this issue said the tool "exits
successfully while reporting nothing" and made that the reason the rot hid. That is WRONG, and I
inferred it from the module docstring ("no pass/fail — it *measures*") without reading the exit path.
`bin/data-quality` in fact handles degradation properly: every failed query is named on stderr, sets
`degraded = true`, and the process returns `ExitCode::FAILURE`. A scheduled run would NOT have looked
fine.

The real reason it went unnoticed is duller and more actionable: **nothing runs it.** It is a manual
tool, invoked by a person when someone thinks to, and nobody had since the corpus outgrew the queries.
So the project has had no semantic-quality measurement for however long that is, while believing issue
27 delivered one — not because the signal lied, but because no one was listening for it.

One real conflation does survive, in the RENDERED output rather than the exit code: section 4 printed
"DÖE procedure Tenders: 0; merged with TED: 0 (—)" for a query that never ran. stdout shows zeros
where it should show "unmeasured", so a reader who sees the report pasted into a comment — without the
stderr lines or the exit code — is misled. That is worth fixing on its own.

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

**Two smaller pieces, independent of which option above wins:**

- ~~The RENDER must distinguish unmeasured from zero~~ **DONE** (`1f9a538`, 2026-08-18): failure is now
  explicit in the type — `from_labelled` takes `Option<Rows>`, `None` meaning the query did not run —
  and `Raw`/`Report` carry the unmeasured labels. The report opens with an `INCOMPLETE: N of 11 queries
  did not run (…)` line and section 4 prints `UNMEASURED — the \`merge\` query did not run` instead of a
  zero. Test drives the exact prod shape (all 11 failed) plus the complement (a query that ran and
  matched nothing still reports its real zero, no banner).
- Something must RUN it. A report nobody invokes cannot rot loudly however good its exit code is —
  which is precisely what happened here. If the measurement moves server-side (option 1) this solves
  itself, since the refresher runs on a cadence; if it stays external, it needs a schedule and somewhere
  for a non-zero exit to land.
