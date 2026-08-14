# 200 — missing-publication-id: 108 old text-era rows + 2 new eForms-adjacent rows

Status: needs-triage
Kind: quarantine bucket diagnosis
Relates to: 195 (the 2 new rows surfaced in its reclaims via the issue-87 rewrite)

## What (measured via /v1/sql, 2026-08-14)

110 outstanding `missing-publication-id` rows: 108 profile `text`, detail NULL (the old
bucket — text-era records without an ND line, never diagnosed) and 2 that re-held under
this reason during the 195 reclaims (their members re-parse to records lacking a
publication id — need member extraction to see whether the id is genuinely absent in the
source or lost by the parser). No ledger entry names this population yet — add one
(resolved or outstanding) once diagnosed.

## Next

Group the 108 by member/package vintage; extract the 2 new members (query: reason =
missing-publication-id, attempts > 0).
