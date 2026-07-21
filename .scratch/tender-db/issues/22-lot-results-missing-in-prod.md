# 22 — lot_results empty in production despite CANs

Status: resolved

## Resolution (2026-07-21, team lead)

The 2026-07-20 zero was projection-run state, not a code gap. Evidence:
prod (rev bad8dda) holds 12,600 lot_results / 23,987 bids / 14,767
contracts for 7,163 Tenders, and issue 27's data-quality measurement
shows **results materialise at 100% on the projected layer** (award
notices → lot_results density, measured live 2026-07-21). The original
observation predated any projection run over those CANs. The era-ladder
eForms-CAN check re-confirms as part of the standard post-backfill
`verify` run (issue 15); no separate work remains.

Verify-agent finding (2026-07-20, prod rev 7199300): 2,529 subtype-29
eForms CANs present but zero `lot_results` rows — the era-ladder check
fails on "eForms CAN carries a winner". Issue 13 is marked resolved, so
either its projection never ran against this data (plausible: the queued
job 5 `project rebuild=false` will materialise them) or there is a real
projection gap in prod.

Task: after the issue-15 run's project job completes, check
`lot_results` counts vs CAN counts in prod. If still zero/short,
diagnose the projection path for results (root cause, not a re-run
band-aid) and fix. If the project job fixed it, verify the era-ladder
check passes and close with the counts.

Acceptance: era-ladder eForms-CAN check green against production;
counts recorded on this issue.
