# 22 — lot_results empty in production despite CANs

Status: ready-for-agent
Blocked by: 15 (job 5 `project` must finish first — it may be the fix)

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
