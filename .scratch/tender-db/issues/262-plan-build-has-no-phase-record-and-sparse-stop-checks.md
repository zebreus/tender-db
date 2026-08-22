# 262 — the fold's plan build shows `phase: None` and honours a stop only ~17 min later

Status: CLOSED — fixed 2026-08-21, live-confirmed 2026-08-22. — the incremental's plan-build passes now emit
`Progress::Planning` per chunk (each pass over its own total) and poll the stop flag per chunk, the
grouping boundary emits `Grouped`, both phase-2 arms forward `Applying`, and the supervisor maps it
all into the durable phase record via `project_incremental_observed_stoppable` (the same
`phase_from_progress` the full path uses). A stop during pass 2 clears the partial plan and — the
part the new gate caught being wrong on the first cut — does NOT advance the legacy-adjacency
watermark, whose attestation only a completed build earns. Gate:
`the_incremental_plan_build_reports_progress_and_stops_within_a_chunk` (three scenarios: progress
surfaces; immediate stop plans nothing; genuine mid-pass-2 stop leaves the watermark). COMPLETED
same day with the fallback corner: the INCREMENTAL → FULL fallback now threads the caller's sink
alongside stderr (it had swapped in a stderr-only one, so the biggest folds — era-scale deltas
whose closure exceeds the cap, the r208/r209 shape exactly — would STILL have run dark), gated by
`the_full_fallback_still_surfaces_the_callers_progress`. LIVE-CONFIRMED 2026-08-22 06:50 UTC:
campaign fold 311 (a 389k-notice delta) shows `planning — notices planned` during its plan build —
the stage that was `phase: None` for job 306's entire multi-hour walk two days earlier. The
cancel-latency half is structural (the stop check sits at every chunk boundary the progress event
marks) and needs no manufactured cancel to prove again.
Kind: observability + cancel-latency rough edge
Blocked by: —
Relates to: 256 (whose probe measured it), 65 (the phase record this stage never sets), 252
(honest cancel — the honesty is there, the latency and blindness are what remain)

## What the probe showed

Cancelling fold job 303 mid-plan-build (a 2.7M-notice delta from the r208 re-parse):

- The route and journal answered honestly and immediately (`state: stopping`, "asked it to stop
  at its next checkpoint") — issue 252's half works.
- But the job's phase record read `None` for the ENTIRE plan-build stage — an operator watching
  /admin/jobs sees a running job with no phase, no progress, no detail for tens of minutes. The
  phase machinery (issue 65) starts only at grouping/fold time.
- And the stop was honoured ~17 minutes after the ack: the plan build's stop checks sit at
  boundaries that are far apart at multi-million-notice delta scale.

## Fix shape (small)

1. `set_phase("planning", Some(notices_scanned), Some(delta_total), …)` from the plan build's
   batch loop — the loop already counts what it scans; the phase record is one call.
2. Check the stop flag at every plan-build batch boundary (it may already be checked at some — 
   establish the actual interval and tighten to the batch loop; target: a stop honoured within
   ~1 min at any scale).

Acceptance: a cancelled fold in plan build shows a `planning` phase with moving numbers, and the
`CANCELLED at a checkpoint` row lands within ~a minute of the ack.
