# 262 — the fold's plan build shows `phase: None` and honours a stop only ~17 min later

Status: needs-triage — filed 2026-08-21 (owner) out of issue 256's live cancel probe.
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
