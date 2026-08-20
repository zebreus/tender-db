# 253 — the data-quality `awards` query is ~11× slower now that the text era actually has awards

Status: needs-triage, filed 2026-08-20 (owner) — one measurement, cause NOT yet diagnosed; the
diagnosis needs a bounded A/B on one window with the queue idle
Kind: performance regression in a scheduled job, caused by the data improving
Blocked by: —
Relates to: 243 (which merged three award queries into this one and measured its cost), 230 (the
windowed measurement), 244 (the campaign that gave the era its awards), 235 (the numbers the query
feeds), 239/248 (the two turso planner gaps already measured on this corpus)

## What

Issue 243 measured the merged `awards` query across a full run: the three separate forms cost
1,346 s + 495 s + 813 s over 32 windows, and the merged one about 30 % less — call it **~58 s per
window**. The whole run was 5,566 s (92.8 min).

Watched live on 2026-08-20, the same query on **window 2 of 32 took ~10–12 minutes**. That is ~11× its
measured per-window cost. The queries around it are unremarkable: 11 other queries went by in the
following 6 minutes (~30 s each), so this is one query, not a box-wide slowdown. The server was at
51 % CPU throughout — working, not stalled.

## The obvious suspect, and why it is only a suspect

The text-era campaign (issue 244) is the thing that changed. Before it, legacy award notices projected
**no** result content: 0.3 % of a package's award notices carried a winner. They now carry 91.7 %, which
means `lot_results`, `notice_sections` of kind `LotResult`, and the winner organizations all grew by
millions of rows — in exactly the tender-id ranges the early windows cover.

The query does two per-row EXISTS probes and one document-type probe:

    EXISTS(SELECT 1 FROM lot_results lr WHERE lr.tender_id = tv.tender_id
                                          AND lr.notice_id = tv.caused_by_notice_id)
    NOT EXISTS(SELECT 1 FROM notice_sections s WHERE s.notice_id = tv.caused_by_notice_id
                                                 AND s.kind IN ('LotResult','TenderResult'))

`lot_results` has `UNIQUE(tender_id, notice_id, result_key)`, whose prefix should serve the first probe,
and `notice_sections` is keyed `(notice_id, section_id)` for the second. So **on paper both seek** — but
this engine has already been measured twice not using an index the shape of the query implies
(issue 248: a DELETE ignored the implicit composite-PK index and scanned 41.78M rows while the same
predicate as a SELECT seeked in 0.8 ms; issue 239: no predicate pushdown into any view). Which of the
three probes actually degraded is unknown, and guessing is how the last three wrong diagnoses on issue
247 happened.

## What to do, in order

1. **Measure, don't guess.** With the queue idle, time each of the three probes separately over one
   legacy window via `/v1/sql`, and compare against a high-numbered (eForms) window. `--print-sql` on
   the data-quality binary already emits the instantiated statements, so this needs no new code.
2. Only then decide between an index, a rewrite (e.g. a join over a pre-aggregated `lot_results` set
   instead of a per-row EXISTS), or accepting a slower run with the window count raised.
3. **A/B whatever lands**, on one window, before the next full run — the discipline issue 243
   established for this exact query.

## Why it matters beyond the report

The run holds the job queue. At the observed rate the early (legacy) windows alone would take hours,
which delays the 09:35 daily tick behind it and makes `ingest_freshness` read stale for no good reason.
Issue 252 is the other half of that problem: the run cannot currently be stopped once started.

## Acceptance

- The three probes timed separately on a legacy window and on an eForms window, recorded here.
- A named cause, or an explicit "measured and it is inherent" finding.
- If a fix lands: an A/B on one window, and the next full run's total recorded against the 5,566 s
  baseline.
